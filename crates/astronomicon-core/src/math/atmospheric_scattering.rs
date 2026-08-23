use crate::chemistry::optics::GasOpticalProperties;
use crate::math::aerosol::{
    mie_scattering_coefficient_at_wavelength,
    AtmosphericAerosolProperties,
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
use crate::units::{ Angle, ColorRGB, Length, Pressure, Temperature, Vector3, Wavelength };
use serde::{ Deserialize, Serialize };
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericRaymarchConfig {
    pub view_samples: u32,
    pub sun_samples: u32,
    pub atmosphere_top_altitude: Length,
}

impl AtmosphericRaymarchConfig {
    pub fn new(view_samples: u32, sun_samples: u32, atmosphere_top_altitude: Length) -> Self {
        Self {
            view_samples: view_samples.max(4),
            sun_samples: sun_samples.max(2),
            atmosphere_top_altitude,
        }
    }

    pub fn fast() -> Self {
        Self {
            view_samples: 16,
            sun_samples: 8,
            atmosphere_top_altitude: Length::new(100_000.0),
        }
    }

    pub fn accurate() -> Self {
        Self {
            view_samples: 64,
            sun_samples: 32,
            atmosphere_top_altitude: Length::new(100_000.0),
        }
    }
}

impl Default for AtmosphericRaymarchConfig {
    fn default() -> Self {
        Self {
            view_samples: 32,
            sun_samples: 16,
            atmosphere_top_altitude: Length::new(100_000.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SphericalAtmosphere {
    pub planet_radius: Length,
    pub atmosphere_top_radius: Length,
    pub surface_pressure: Pressure,
    pub surface_temperature: Temperature,
    pub gas_scale_height: Length,
    pub aerosol_scale_height: Length,
    pub gas_optical_properties: GasOpticalProperties,
    pub aerosol_properties: AtmosphericAerosolProperties,
}

impl SphericalAtmosphere {
    pub fn new(
        planet_radius: Length,
        atmosphere_top_altitude: Length,
        surface_pressure: Pressure,
        surface_temperature: Temperature,
        gas_scale_height: Length,
        aerosol_scale_height: Length,
        gas_optical_properties: GasOpticalProperties,
        aerosol_properties: AtmosphericAerosolProperties
    ) -> Self {
        let top_r = Length::new(
            planet_radius.value() + atmosphere_top_altitude.value().max(1000.0)
        );
        Self {
            planet_radius,
            atmosphere_top_radius: top_r,
            surface_pressure,
            surface_temperature,
            gas_scale_height,
            aerosol_scale_height,
            gas_optical_properties,
            aerosol_properties,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SphericalOpticalDepth {
    pub gas_depth: f64,
    pub aerosol_depth: f64,
}

impl SphericalOpticalDepth {
    pub fn new(gas_depth: f64, aerosol_depth: f64) -> Self {
        Self {
            gas_depth: gas_depth.max(0.0),
            aerosol_depth: aerosol_depth.max(0.0),
        }
    }

    pub fn zero() -> Self {
        Self {
            gas_depth: 0.0,
            aerosol_depth: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SphericalScatteringResult {
    pub in_scattered_radiance: f64,
    pub optical_depth: f64,
    pub transmittance: f64,
}

pub fn ray_sphere_intersections(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sphere_radius: Length
) -> Option<(Length, Length)> {
    let r = sphere_radius.value();
    let d = ray_dir.normalized();

    if r <= 0.0 || !r.is_finite() {
        return None;
    }

    let b = ray_origin.dot(&d);
    let c = ray_origin.dot(&ray_origin) - r * r;
    let disc = b * b - c;

    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();
    let t1 = -b - sqrt_disc;
    let t2 = -b + sqrt_disc;

    Some((Length::new(t1), Length::new(t2)))
}

pub fn ray_atmosphere_segment(
    ray_origin: Vector3,
    ray_dir: Vector3,
    planet_radius: Length,
    atmosphere_top_radius: Length
) -> Option<(Length, Length, bool)> {
    let r_p = planet_radius.value();
    let r_atm = atmosphere_top_radius.value();
    let d = ray_dir.normalized();

    if r_p <= 0.0 || r_atm <= r_p || !r_p.is_finite() || !r_atm.is_finite() {
        return None;
    }

    let r0_sq = ray_origin.dot(&ray_origin);
    let r0 = r0_sq.sqrt();

    let b_atm = ray_origin.dot(&d);
    let c_atm = r0_sq - r_atm * r_atm;
    let disc_atm = b_atm * b_atm - c_atm;

    if disc_atm < 0.0 {
        return None;
    }

    let sqrt_disc_atm = disc_atm.sqrt();
    let t_atm1 = -b_atm - sqrt_disc_atm;
    let t_atm2 = -b_atm + sqrt_disc_atm;

    if t_atm2 <= 0.0 {
        return None;
    }

    let t_start = t_atm1.max(0.0);
    let mut t_end = t_atm2;
    let mut hits_ground = false;

    let b_p = ray_origin.dot(&d);
    let c_p = r0_sq - r_p * r_p;
    let disc_p = b_p * b_p - c_p;

    if disc_p >= 0.0 {
        let sqrt_disc_p = disc_p.sqrt();
        let t_p1 = -b_p - sqrt_disc_p;
        let t_p2 = -b_p + sqrt_disc_p;

        if t_p1 > 1e-6 {
            if t_p1 < t_end {
                t_end = t_p1;
                hits_ground = true;
            }
        } else if t_p2 > 1e-6 && b_p < 0.0 {
            if r0 <= r_p + 1.0 {
                return None;
            }
            t_end = 0.0;
            hits_ground = true;
        }
    }

    if t_end <= t_start + 1e-6 {
        return None;
    }

    Some((Length::new(t_start), Length::new(t_end), hits_ground))
}

pub fn spherical_optical_depth_segment(
    start_pos: Vector3,
    end_pos: Vector3,
    planet_radius: Length,
    gas_scale_height: Length,
    aerosol_scale_height: Length,
    samples: u32
) -> SphericalOpticalDepth {
    let diff = end_pos - start_pos;
    let dist = diff.magnitude();

    if dist <= 0.0 || !dist.is_finite() {
        return SphericalOpticalDepth::zero();
    }

    let n = samples.max(2);
    let ds = dist / (n as f64);
    let dir = diff / dist;

    let r_p = planet_radius.value();
    let h_gas = gas_scale_height.value().max(1.0);
    let h_aero = aerosol_scale_height.value().max(1.0);

    let mut tau_gas = 0.0;
    let mut tau_aero = 0.0;

    for i in 0..n {
        let s = ((i as f64) + 0.5) * ds;
        let p = start_pos + dir * s;
        let r = p.magnitude();
        let alt = (r - r_p).max(0.0);

        let exp_gas = -alt / h_gas;
        let exp_aero = -alt / h_aero;

        if exp_gas >= -700.0 {
            tau_gas += exp_gas.exp() * ds;
        }
        if exp_aero >= -700.0 {
            tau_aero += exp_aero.exp() * ds;
        }
    }

    SphericalOpticalDepth::new(tau_gas, tau_aero)
}

pub fn sun_path_optical_depth(
    sample_pos: Vector3,
    sun_dir: Vector3,
    planet_radius: Length,
    atmosphere_top_radius: Length,
    gas_scale_height: Length,
    aerosol_scale_height: Length,
    samples: u32
) -> Option<SphericalOpticalDepth> {
    let s = sun_dir.normalized();
    let r_p = planet_radius.value();
    let r_atm = atmosphere_top_radius.value();

    if r_p <= 0.0 || r_atm <= r_p {
        return None;
    }

    let r_sq = sample_pos.dot(&sample_pos);
    let b_p = sample_pos.dot(&s);
    let c_p = r_sq - r_p * r_p;
    let disc_p = b_p * b_p - c_p;

    if disc_p >= 0.0 {
        let sqrt_disc_p = disc_p.sqrt();
        let t_p1 = -b_p - sqrt_disc_p;
        let t_p2 = -b_p + sqrt_disc_p;

        if t_p1 > 1e-4 {
            return None;
        }
        if t_p2 > 1e-4 && b_p < 0.0 {
            return None;
        }
    }

    let b_atm = sample_pos.dot(&s);
    let c_atm = r_sq - r_atm * r_atm;
    let disc_atm = b_atm * b_atm - c_atm;

    if disc_atm < 0.0 {
        return Some(SphericalOpticalDepth::zero());
    }

    let t_exit = -b_atm + disc_atm.sqrt();
    if t_exit <= 1e-6 {
        return Some(SphericalOpticalDepth::zero());
    }

    let n = samples.max(2);
    let ds = t_exit / (n as f64);
    let h_gas = gas_scale_height.value().max(1.0);
    let h_aero = aerosol_scale_height.value().max(1.0);

    let mut tau_gas = 0.0;
    let mut tau_aero = 0.0;

    for j in 0..n {
        let step = ((j as f64) + 0.5) * ds;
        let p_j = sample_pos + s * step;
        let r_j = p_j.magnitude();
        let alt_j = (r_j - r_p).max(0.0);

        let exp_gas = -alt_j / h_gas;
        let exp_aero = -alt_j / h_aero;

        if exp_gas >= -700.0 {
            tau_gas += exp_gas.exp() * ds;
        }
        if exp_aero >= -700.0 {
            tau_aero += exp_aero.exp() * ds;
        }
    }

    Some(SphericalOpticalDepth::new(tau_gas, tau_aero))
}

pub fn single_scattering_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig
) -> SphericalScatteringResult {
    if solar_spectral_irradiance <= 0.0 || !solar_spectral_irradiance.is_finite() {
        return SphericalScatteringResult {
            in_scattered_radiance: 0.0,
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
            return SphericalScatteringResult {
                in_scattered_radiance: 0.0,
                optical_depth: 0.0,
                transmittance: 1.0,
            };
        }
    };

    let total_path_len = t_end.value() - t_start.value();
    if total_path_len <= 0.0 || !total_path_len.is_finite() {
        return SphericalScatteringResult {
            in_scattered_radiance: 0.0,
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

    let n_view = config.view_samples.max(4);
    let ds = total_path_len / (n_view as f64);

    let mut tau_view_gas = 0.0;
    let mut tau_view_aero = 0.0;
    let mut in_scatter_accum = 0.0;

    let r_p = atmosphere.planet_radius.value();
    let r_top = atmosphere.atmosphere_top_radius.value();
    let h_gas = atmosphere.gas_scale_height.value().max(1.0);
    let h_aero = atmosphere.aerosol_scale_height.value().max(1.0);

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

        let sun_depth = sun_path_optical_depth(
            pos_i,
            s_dir,
            atmosphere.planet_radius,
            atmosphere.atmosphere_top_radius,
            atmosphere.gas_scale_height,
            atmosphere.aerosol_scale_height,
            config.sun_samples
        );

        if let Some(sun_tau) = sun_depth {
            let total_tau =
                beta_e_r0 * (tau_view_gas + sun_tau.gas_depth) +
                beta_e_m0 * (tau_view_aero + sun_tau.aerosol_depth);

            let attenuation = if total_tau > 700.0 { 0.0 } else { (-total_tau).exp() };

            let scatter_coeff = beta_s_r0 * rho_g * phase_r + beta_s_m0 * rho_a * phase_m;
            in_scatter_accum += solar_spectral_irradiance * attenuation * scatter_coeff * ds;
        }
    }

    let ray_total_tau = beta_e_r0 * tau_view_gas + beta_e_m0 * tau_view_aero;
    let transmittance = if ray_total_tau > 700.0 {
        0.0
    } else {
        (-ray_total_tau).exp().clamp(0.0, 1.0)
    };

    SphericalScatteringResult {
        in_scattered_radiance: in_scatter_accum.max(0.0),
        optical_depth: ray_total_tau.max(0.0),
        transmittance,
    }
}

pub fn spherical_sky_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    planet_radius: Length,
    surface_pressure: Pressure,
    surface_temperature: Temperature,
    gas_scale_height: Length,
    aerosol_scale_height: Length,
    gas_props: &GasOpticalProperties,
    aerosol_props: &AtmosphericAerosolProperties,
    config: &AtmosphericRaymarchConfig
) -> SphericalScatteringResult {
    let atmosphere = SphericalAtmosphere::new(
        planet_radius,
        config.atmosphere_top_altitude,
        surface_pressure,
        surface_temperature,
        gas_scale_height,
        aerosol_scale_height,
        gas_props.clone(),
        *aerosol_props
    );

    single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_spectral_irradiance,
        wavelength,
        &atmosphere,
        config
    )
}

pub fn spherical_sky_color_xyz(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig
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
            let result = single_scattering_spectral_radiance(
                ray_origin,
                ray_dir,
                sun_dir,
                solar_irradiance,
                wavelength,
                atmosphere,
                config
            );

            let cmf = cie_color_matching_functions(wavelength);
            accumulated = accumulated + cmf * result.in_scattered_radiance;
        }

        lambda_m += step;
    }

    accumulated * step
}

pub fn spherical_sky_color_rgb(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    exposure: f64,
    config: &AtmosphericRaymarchConfig
) -> ColorRGB {
    let xyz = spherical_sky_color_xyz(
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

pub fn spherical_sky_rgb_fast(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_irradiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig
) -> (ColorRGB, ColorRGB) {
    let w_r = Wavelength::new(680.0e-9);
    let w_g = Wavelength::new(550.0e-9);
    let w_b = Wavelength::new(440.0e-9);

    let res_r = single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.r(),
        w_r,
        atmosphere,
        config
    );

    let res_g = single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.g(),
        w_g,
        atmosphere,
        config
    );

    let res_b = single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.b(),
        w_b,
        atmosphere,
        config
    );

    let in_scattered = ColorRGB::new(
        res_r.in_scattered_radiance,
        res_g.in_scattered_radiance,
        res_b.in_scattered_radiance
    );

    let transmittance = ColorRGB::new(
        res_r.transmittance,
        res_g.transmittance,
        res_b.transmittance
    );

    (in_scattered, transmittance)
}

pub fn spherical_view_transmittance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig
) -> f64 {
    let v_dir = ray_dir.normalized();
    let segment = ray_atmosphere_segment(
        ray_origin,
        v_dir,
        atmosphere.planet_radius,
        atmosphere.atmosphere_top_radius
    );

    let (t_start, t_end, _hits_ground) = match segment {
        Some(seg) => seg,
        None => {
            return 1.0;
        }
    };

    let total_path_len = t_end.value() - t_start.value();
    if total_path_len <= 0.0 || !total_path_len.is_finite() {
        return 1.0;
    }

    let depth = spherical_optical_depth_segment(
        ray_origin + v_dir * t_start.value(),
        ray_origin + v_dir * t_end.value(),
        atmosphere.planet_radius,
        atmosphere.gas_scale_height,
        atmosphere.aerosol_scale_height,
        config.view_samples
    );

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

    let total_tau = beta_e_r0 * depth.gas_depth + beta_e_m0 * depth.aerosol_depth;
    if total_tau > 700.0 {
        0.0
    } else {
        (-total_tau).exp().clamp(0.0, 1.0)
    }
}
