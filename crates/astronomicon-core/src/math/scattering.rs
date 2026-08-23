use crate::math::aerosol::mie_scattering_coefficient_at_wavelength;
use crate::math::atmospheric_scattering::{
    ray_atmosphere_segment,
    sun_path_optical_depth,
    SphericalAtmosphere,
};
use crate::math::colorimetry::{
    cie_color_matching_functions,
    exposure_tone_map,
    linear_to_srgb_gamma,
    xyz_to_linear_srgb,
    ColorXYZ,
};
use crate::math::optics::{
    absorption_coefficient,
    henyey_greenstein_phase_function,
    mie_scattering_coefficient,
    rayleigh_phase_function_with_depolarization,
    rayleigh_scattering_coefficient,
};
use crate::math::radiation::planck_spectral_radiance;
use crate::units::constants::{
    CIE_WAVELENGTH_MAX_M,
    CIE_WAVELENGTH_MIN_M,
    CIE_WAVELENGTH_STEP_M,
    OPTICAL_REFERENCE_WAVELENGTH,
};
use crate::units::{ Angle, ColorRGB, Length, Temperature, Vector3, Wavelength };
use serde::{ Deserialize, Serialize };
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MultipleScatteringConfig {
    pub view_samples: u32,
    pub sun_samples: u32,
    pub atmosphere_top_altitude: Length,
    pub ground_albedo: f64,
    pub multiple_scattering_factor: f64,
}

impl MultipleScatteringConfig {
    pub fn new(
        view_samples: u32,
        sun_samples: u32,
        atmosphere_top_altitude: Length,
        ground_albedo: f64,
        multiple_scattering_factor: f64
    ) -> Self {
        Self {
            view_samples: view_samples.max(4),
            sun_samples: sun_samples.max(2),
            atmosphere_top_altitude,
            ground_albedo: ground_albedo.clamp(0.0, 1.0),
            multiple_scattering_factor: multiple_scattering_factor.clamp(0.0, 5.0),
        }
    }

    pub fn fast() -> Self {
        Self {
            view_samples: 16,
            sun_samples: 8,
            atmosphere_top_altitude: Length::new(100_000.0),
            ground_albedo: 0.15,
            multiple_scattering_factor: 1.0,
        }
    }

    pub fn accurate() -> Self {
        Self {
            view_samples: 64,
            sun_samples: 32,
            atmosphere_top_altitude: Length::new(100_000.0),
            ground_albedo: 0.15,
            multiple_scattering_factor: 1.0,
        }
    }

    pub fn with_ground_albedo(mut self, ground_albedo: f64) -> Self {
        self.ground_albedo = ground_albedo.clamp(0.0, 1.0);
        self
    }

    pub fn with_multiple_scattering_factor(mut self, factor: f64) -> Self {
        self.multiple_scattering_factor = factor.clamp(0.0, 5.0);
        self
    }
}

impl Default for MultipleScatteringConfig {
    fn default() -> Self {
        Self {
            view_samples: 32,
            sun_samples: 16,
            atmosphere_top_altitude: Length::new(100_000.0),
            ground_albedo: 0.15,
            multiple_scattering_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MultipleScatteringResult {
    pub single_scattered_radiance: f64,
    pub multiple_scattered_radiance: f64,
    pub total_radiance: f64,
    pub optical_depth: f64,
    pub transmittance: f64,
}

pub fn ground_solid_angle_factor(altitude: Length, planet_radius: Length) -> f64 {
    let r = planet_radius.value();
    let z = altitude.value().max(0.0);

    if r <= 0.0 || !r.is_finite() || !z.is_finite() {
        return 0.0;
    }

    let r_plus_z = r + z;
    if r_plus_z <= 0.0 {
        return 0.0;
    }

    let disc = z * (2.0 * r + z);
    let cos_horizon = disc.max(0.0).sqrt() / r_plus_z;
    let factor = 0.5 * (1.0 - cos_horizon.clamp(0.0, 1.0));

    factor.clamp(0.0, 0.5)
}

pub fn single_scattering_albedo(scattering_coefficient: f64, extinction_coefficient: f64) -> f64 {
    if extinction_coefficient <= 0.0 || !extinction_coefficient.is_finite() {
        return 0.0;
    }
    (scattering_coefficient / extinction_coefficient).clamp(0.0, 1.0)
}

pub fn multiple_scattering_transfer_factor(optical_depth: f64) -> f64 {
    if optical_depth <= 0.0 || !optical_depth.is_finite() {
        return 0.0;
    }
    if optical_depth > 700.0 {
        1.0
    } else {
        (1.0 - (-optical_depth).exp()).clamp(0.0, 1.0)
    }
}

pub fn ground_reflected_radiance(incident_irradiance: f64, ground_albedo: f64) -> f64 {
    let albedo = ground_albedo.clamp(0.0, 1.0);
    let irr = incident_irradiance.max(0.0);
    if !irr.is_finite() || irr <= 0.0 {
        0.0
    } else {
        (albedo / PI) * irr
    }
}

pub fn isotropic_multiple_scattering_source(
    direct_irradiance: f64,
    ground_irradiance: f64,
    ssa: f64,
    optical_depth: f64,
    multiple_scattering_factor: f64
) -> f64 {
    let f_dir = direct_irradiance.max(0.0);
    let f_ground = ground_irradiance.max(0.0);
    let total_flux = f_dir + f_ground;

    if total_flux <= 0.0 || !total_flux.is_finite() || ssa <= 0.0 || !ssa.is_finite() {
        return 0.0;
    }

    let f_ms =
        multiple_scattering_transfer_factor(optical_depth) *
        multiple_scattering_factor.clamp(0.0, 5.0);
    let j1 = (ssa / (4.0 * PI)) * total_flux;
    let order_2_factor = ssa * f_ms;
    let denom = (1.0 - order_2_factor).max(0.005);
    let j_ms = j1 * (order_2_factor / denom);

    if !j_ms.is_finite() || j_ms < 0.0 {
        0.0
    } else {
        j_ms
    }
}

pub fn multiple_scattering_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig
) -> MultipleScatteringResult {
    if solar_spectral_irradiance <= 0.0 || !solar_spectral_irradiance.is_finite() {
        return MultipleScatteringResult {
            single_scattered_radiance: 0.0,
            multiple_scattered_radiance: 0.0,
            total_radiance: 0.0,
            optical_depth: 0.0,
            transmittance: 1.0,
        };
    }

    let v_dir = ray_dir.normalized();
    let s_dir = sun_dir.normalized();

    let segment = ray_atmosphere_segment(
        ray_origin,
        v_dir,
        atmosphere.planet_radius,
        atmosphere.atmosphere_top_radius
    );

    let (t_start, t_end, _hits_ground) = match segment {
        Some(seg) => seg,
        None => {
            return MultipleScatteringResult {
                single_scattered_radiance: 0.0,
                multiple_scattered_radiance: 0.0,
                total_radiance: 0.0,
                optical_depth: 0.0,
                transmittance: 1.0,
            };
        }
    };

    let total_path_len = t_end.value() - t_start.value();
    if total_path_len <= 0.0 || !total_path_len.is_finite() {
        return MultipleScatteringResult {
            single_scattered_radiance: 0.0,
            multiple_scattered_radiance: 0.0,
            total_radiance: 0.0,
            optical_depth: 0.0,
            transmittance: 1.0,
        };
    }

    let beta_s_r0 = rayleigh_scattering_coefficient(
        wavelength,
        atmosphere.gas_optical_properties.refractivity_stp(),
        atmosphere.gas_optical_properties.king_factor(),
        atmosphere.surface_pressure,
        atmosphere.surface_temperature
    );

    let beta_a_g0 = absorption_coefficient(
        &atmosphere.gas_optical_properties,
        wavelength,
        atmosphere.surface_pressure,
        atmosphere.surface_temperature
    );

    let beta_e_r0 = beta_s_r0 + beta_a_g0;

    let ref_wavelength = Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH);
    let beta_s_m0 = mie_scattering_coefficient(
        &atmosphere.aerosol_properties,
        wavelength,
        ref_wavelength
    );

    let beta_e_m0 = mie_scattering_coefficient_at_wavelength(
        atmosphere.aerosol_properties.base_extinction_coefficient(),
        wavelength,
        ref_wavelength,
        atmosphere.aerosol_properties.angstrom_exponent()
    ).max(beta_s_m0);

    let cos_theta = v_dir.dot(&s_dir).clamp(-1.0, 1.0);
    let theta = Angle::new(cos_theta.acos());
    let phase_r = rayleigh_phase_function_with_depolarization(
        theta,
        atmosphere.gas_optical_properties.king_factor()
    );
    let phase_m = henyey_greenstein_phase_function(
        theta,
        atmosphere.aerosol_properties.asymmetry_factor_g()
    );

    let r_p = atmosphere.planet_radius.value();
    let r_top = atmosphere.atmosphere_top_radius.value();
    let h_gas = atmosphere.gas_scale_height.value().max(1.0);
    let h_aero = atmosphere.aerosol_scale_height.value().max(1.0);

    let subsolar_ground_pos = s_dir * r_p;
    let ground_sun_depth = sun_path_optical_depth(
        subsolar_ground_pos,
        s_dir,
        atmosphere.planet_radius,
        atmosphere.atmosphere_top_radius,
        atmosphere.gas_scale_height,
        atmosphere.aerosol_scale_height,
        config.sun_samples
    );

    let ground_incident_irradiance = match ground_sun_depth {
        Some(sd) => {
            let tau_g = beta_e_r0 * sd.gas_depth + beta_e_m0 * sd.aerosol_depth;
            if tau_g > 700.0 {
                0.0
            } else {
                solar_spectral_irradiance * (-tau_g).exp()
            }
        }
        None => 0.0,
    };

    let l_ground = ground_reflected_radiance(ground_incident_irradiance, config.ground_albedo);

    let n_view = config.view_samples.max(4);
    let ds = total_path_len / (n_view as f64);

    let mut tau_view_gas = 0.0;
    let mut tau_view_aero = 0.0;
    let mut in_scatter_ss = 0.0;
    let mut in_scatter_ms = 0.0;

    for i in 0..n_view {
        let s_i = t_start.value() + ((i as f64) + 0.5) * ds;
        let pos_i = ray_origin + v_dir * s_i;
        let r_i = pos_i.magnitude();
        let alt_i = (r_i - r_p).max(0.0);

        if alt_i > r_top - r_p {
            continue;
        }

        let exp_gas = -alt_i / h_gas;
        let exp_aero = -alt_i / h_aero;

        let rho_g = if exp_gas >= -700.0 { exp_gas.exp() } else { 0.0 };
        let rho_a = if exp_aero >= -700.0 { exp_aero.exp() } else { 0.0 };

        tau_view_gas += rho_g * ds;
        tau_view_aero += rho_a * ds;

        let beta_s_loc = beta_s_r0 * rho_g + beta_s_m0 * rho_a;
        let beta_e_loc = beta_e_r0 * rho_g + beta_e_m0 * rho_a;
        let ssa_loc = single_scattering_albedo(beta_s_loc, beta_e_loc);

        let current_view_tau = beta_e_r0 * tau_view_gas + beta_e_m0 * tau_view_aero;
        let view_transmittance = if current_view_tau > 700.0 {
            0.0
        } else {
            (-current_view_tau).exp()
        };

        let sun_depth = sun_path_optical_depth(
            pos_i,
            s_dir,
            atmosphere.planet_radius,
            atmosphere.atmosphere_top_radius,
            atmosphere.gas_scale_height,
            atmosphere.aerosol_scale_height,
            config.sun_samples
        );

        let (dir_irradiance_at_pos, sun_transmittance) = match sun_depth {
            Some(sun_tau) => {
                let tau_sun = beta_e_r0 * sun_tau.gas_depth + beta_e_m0 * sun_tau.aerosol_depth;
                let trans = if tau_sun > 700.0 { 0.0 } else { (-tau_sun).exp() };
                (solar_spectral_irradiance * trans, trans)
            }
            None => (0.0, 0.0),
        };

        if sun_transmittance > 0.0 {
            let scatter_coeff = beta_s_r0 * rho_g * phase_r + beta_s_m0 * rho_a * phase_m;
            in_scatter_ss +=
                solar_spectral_irradiance *
                sun_transmittance *
                scatter_coeff *
                view_transmittance *
                ds;
        }

        let w_ground = ground_solid_angle_factor(Length::new(alt_i), atmosphere.planet_radius);
        let tau_g_to_z = beta_e_r0 * h_gas * (1.0 - rho_g) + beta_e_m0 * h_aero * (1.0 - rho_a);
        let trans_g_to_z = if tau_g_to_z > 700.0 { 0.0 } else { (-tau_g_to_z).exp() };
        let ground_irradiance_at_pos = l_ground * 2.0 * PI * w_ground * trans_g_to_z;

        let tau_top = beta_e_r0 * h_gas * rho_g + beta_e_m0 * h_aero * rho_a;
        let j_ms = isotropic_multiple_scattering_source(
            dir_irradiance_at_pos,
            ground_irradiance_at_pos,
            ssa_loc,
            tau_top,
            config.multiple_scattering_factor
        );

        in_scatter_ms += j_ms * beta_s_loc * view_transmittance * ds;
    }

    let ray_total_tau = beta_e_r0 * tau_view_gas + beta_e_m0 * tau_view_aero;
    let final_transmittance = if ray_total_tau > 700.0 {
        0.0
    } else {
        (-ray_total_tau).exp().clamp(0.0, 1.0)
    };

    let total_rad = in_scatter_ss + in_scatter_ms;

    MultipleScatteringResult {
        single_scattered_radiance: in_scatter_ss.max(0.0),
        multiple_scattered_radiance: in_scatter_ms.max(0.0),
        total_radiance: total_rad.max(0.0),
        optical_depth: ray_total_tau.max(0.0),
        transmittance: final_transmittance,
    }
}

pub fn multiple_scattering_sky_color_xyz(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig
) -> ColorXYZ {
    let t_sun = solar_temperature.value();
    let theta_sun = solar_angular_radius_rad.clamp(0.0, PI / 2.0);

    if t_sun <= 0.0 || theta_sun <= 0.0 || !t_sun.is_finite() || !theta_sun.is_finite() {
        return ColorXYZ::zero();
    }

    let solid_angle_sun = PI * theta_sun.sin().powi(2);
    let step = CIE_WAVELENGTH_STEP_M;
    let mut accumulated = ColorXYZ::zero();
    let mut lambda_m = CIE_WAVELENGTH_MIN_M;

    while lambda_m <= CIE_WAVELENGTH_MAX_M {
        let wavelength = Wavelength::new(lambda_m);
        let b_lambda = planck_spectral_radiance(wavelength, solar_temperature);
        let solar_irradiance = b_lambda * solid_angle_sun;

        if solar_irradiance > 0.0 && solar_irradiance.is_finite() {
            let result = multiple_scattering_spectral_radiance(
                ray_origin,
                ray_dir,
                sun_dir,
                solar_irradiance,
                wavelength,
                atmosphere,
                config
            );

            let cmf = cie_color_matching_functions(wavelength);
            accumulated = accumulated + cmf * result.total_radiance;
        }

        lambda_m += step;
    }

    accumulated * step
}

pub fn multiple_scattering_sky_color_rgb(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    exposure: f64,
    config: &MultipleScatteringConfig
) -> ColorRGB {
    let xyz = multiple_scattering_sky_color_xyz(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_temperature,
        solar_angular_radius_rad,
        atmosphere,
        config
    );
    let linear_rgb = xyz_to_linear_srgb(xyz);
    let exposed = exposure_tone_map(linear_rgb, exposure);
    linear_to_srgb_gamma(exposed)
}

pub fn multiple_scattering_sky_rgb_fast(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_irradiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig
) -> (ColorRGB, ColorRGB, ColorRGB) {
    let w_r = Wavelength::new(680.0e-9);
    let w_g = Wavelength::new(550.0e-9);
    let w_b = Wavelength::new(440.0e-9);

    let res_r = multiple_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.r(),
        w_r,
        atmosphere,
        config
    );

    let res_g = multiple_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.g(),
        w_g,
        atmosphere,
        config
    );

    let res_b = multiple_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.b(),
        w_b,
        atmosphere,
        config
    );

    let single_scattered = ColorRGB::new(
        res_r.single_scattered_radiance,
        res_g.single_scattered_radiance,
        res_b.single_scattered_radiance
    );

    let multiple_scattered = ColorRGB::new(
        res_r.multiple_scattered_radiance,
        res_g.multiple_scattered_radiance,
        res_b.multiple_scattered_radiance
    );

    let transmittance = ColorRGB::new(
        res_r.transmittance,
        res_g.transmittance,
        res_b.transmittance
    );

    (single_scattered, multiple_scattered, transmittance)
}

pub fn multiple_scattering_view_transmittance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig
) -> f64 {
    let res = multiple_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        Vector3::new(0.0, 1.0, 0.0),
        1.0,
        wavelength,
        atmosphere,
        config
    );
    res.transmittance
}
