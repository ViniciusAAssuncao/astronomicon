use crate::chemistry::optics::GasOpticalProperties;
use crate::math::aerosol::refractivity_at_temperature_pressure;
use crate::math::atmospheric_scattering::geometry::ray_atmosphere_segment;
use crate::math::atmospheric_scattering::optical_depth::{
    spherical_optical_depth_segment, sun_path_optical_depth,
};
use crate::math::atmospheric_scattering::profiles::{
    AtmosphericRaymarchConfig, CloudProfile, DustProfile, SphericalAtmosphere, VolcanicProfile,
};
use crate::math::optics::{
    absorption_coefficient, henyey_greenstein_phase_function,
    rayleigh_phase_function_with_depolarization, rayleigh_scattering_coefficient,
    refracted_sun_direction, unrefracted_sun_direction,
};
use crate::math::radiometry::{stellar_disk_sample_directions, stellar_limb_darkening};
use crate::units::{Angle, ColorRGB, Length, Pressure, Temperature, Vector3, Wavelength};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SphericalScatteringResult {
    pub in_scattered_radiance: f64,
    pub optical_depth: f64,
    pub transmittance: f64,
}

pub fn single_scattering_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
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
        atmosphere.atmosphere_top_radius,
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
        atmosphere.surface_temperature,
    );

    let beta_a_g0 = absorption_coefficient(
        &atmosphere.gas_optical_properties,
        wavelength,
        atmosphere.surface_pressure,
        atmosphere.surface_temperature,
    );

    let beta_e_r0 = beta_s_r0 + beta_a_g0;

    let k_s_dust = atmosphere
        .dust_profile
        .scattering_coefficient_at_wavelength(wavelength);
    let k_e_dust = atmosphere
        .dust_profile
        .extinction_coefficient_at_wavelength(wavelength);
    let g_dust = atmosphere.dust_profile.asymmetry_factor_g;

    let k_s_cloud = atmosphere
        .cloud_profile
        .scattering_coefficient_at_wavelength(wavelength);
    let k_e_cloud = atmosphere
        .cloud_profile
        .extinction_coefficient_at_wavelength(wavelength);
    let g_cloud = atmosphere.cloud_profile.asymmetry_factor_g;

    let k_s_volc = atmosphere
        .volcanic_profile
        .scattering_coefficient_at_wavelength(wavelength);
    let k_e_volc = atmosphere
        .volcanic_profile
        .extinction_coefficient_at_wavelength(wavelength);
    let g_volc = atmosphere.volcanic_profile.asymmetry_factor_g;

    let cos_theta = v_dir.dot(&s_dir).clamp(-1.0, 1.0);
    let theta = Angle::new(cos_theta.acos());
    let phase_r = rayleigh_phase_function_with_depolarization(
        theta,
        atmosphere.gas_optical_properties.king_factor(),
    );

    let n_view = config.view_samples.max(4);
    let ds = total_path_len / (n_view as f64);

    let mut tau_view_gas = 0.0;
    let mut tau_view_dust = 0.0;
    let mut tau_view_cloud = 0.0;
    let mut tau_view_volc = 0.0;
    let mut in_scatter_accum = 0.0;

    let r_p = atmosphere.planet_radius.value();
    let r_top = atmosphere.atmosphere_top_radius.value();
    let h_gas = atmosphere.gas_scale_height.value().max(1.0);

    for i in 0..n_view {
        let s_i = t_start.value() + ((i as f64) + 0.5) * ds;
        let pos_i = ray_origin + v_dir * s_i;
        let r_i = pos_i.magnitude();
        let alt_i = (r_i - r_p).max(0.0);

        if alt_i > r_top - r_p {
            continue;
        }

        let exp_gas = -alt_i / h_gas;
        let rho_g = if exp_gas >= -700.0 {
            exp_gas.exp()
        } else {
            0.0
        };

        let alt_len = Length::new(alt_i);
        let rho_d = atmosphere.dust_profile.density_at_altitude(alt_len).value();
        let rho_c = atmosphere
            .cloud_profile
            .density_at_altitude(alt_len)
            .value();
        let rho_v = atmosphere
            .volcanic_profile
            .density_at_altitude(alt_len)
            .value();

        tau_view_gas += rho_g * ds;
        tau_view_dust += rho_d * ds;
        tau_view_cloud += rho_c * ds;
        tau_view_volc += rho_v * ds;

        let b_s_d = rho_d * k_s_dust;
        let b_s_c = rho_c * k_s_cloud;
        let b_s_v = rho_v * k_s_volc;
        let b_s_aero = b_s_d + b_s_c + b_s_v;

        let g_eff = if b_s_aero > 0.0 {
            (b_s_d * g_dust + b_s_c * g_cloud + b_s_v * g_volc) / b_s_aero
        } else {
            0.0
        };

        let phase_m = henyey_greenstein_phase_function(theta, g_eff);

        let sun_depth = sun_path_optical_depth(pos_i, s_dir, atmosphere, config.sun_samples);

        if let Some(sun_tau) = sun_depth {
            let total_tau = beta_e_r0 * (tau_view_gas + sun_tau.gas_depth)
                + k_e_dust * (tau_view_dust + sun_tau.dust_depth)
                + k_e_cloud * (tau_view_cloud + sun_tau.cloud_depth)
                + k_e_volc * (tau_view_volc + sun_tau.volcanic_depth);

            let attenuation = if total_tau > 700.0 {
                0.0
            } else {
                (-total_tau).exp()
            };

            let scatter_coeff = beta_s_r0 * rho_g * phase_r + b_s_aero * phase_m;
            in_scatter_accum += solar_spectral_irradiance * attenuation * scatter_coeff * ds;
        }
    }

    let ray_total_tau = beta_e_r0 * tau_view_gas
        + k_e_dust * tau_view_dust
        + k_e_cloud * tau_view_cloud
        + k_e_volc * tau_view_volc;

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
    gas_props: &GasOpticalProperties,
    dust_profile: DustProfile,
    cloud_profile: CloudProfile,
    volcanic_profile: VolcanicProfile,
    config: &AtmosphericRaymarchConfig,
) -> SphericalScatteringResult {
    let atmosphere = SphericalAtmosphere::new(
        planet_radius,
        config.atmosphere_top_altitude,
        surface_pressure,
        surface_temperature,
        gas_scale_height,
        gas_props.clone(),
        dust_profile,
        cloud_profile,
        volcanic_profile,
    );

    single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_spectral_irradiance,
        wavelength,
        &atmosphere,
        config,
    )
}

pub fn spherical_view_transmittance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> f64 {
    let v_dir = ray_dir.normalized();
    let segment = ray_atmosphere_segment(
        ray_origin,
        v_dir,
        atmosphere.planet_radius,
        atmosphere.atmosphere_top_radius,
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
        atmosphere,
        config.view_samples,
    );

    let total_tau = depth.total_extinction_optical_depth(wavelength, atmosphere);
    if total_tau > 700.0 {
        0.0
    } else {
        (-total_tau).exp().clamp(0.0, 1.0)
    }
}

pub fn refracted_single_scattering_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> SphericalScatteringResult {
    let up = ray_origin.normalized();
    let refr = atmosphere.gas_optical_properties.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(
        refr,
        atmosphere.surface_temperature,
        atmosphere.surface_pressure,
    );
    let s_refracted = refracted_sun_direction(
        geometric_sun_dir,
        up,
        refr_actual,
        atmosphere.gas_scale_height,
        atmosphere.planet_radius,
    );
    single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        s_refracted,
        solar_spectral_irradiance,
        wavelength,
        atmosphere,
        config,
    )
}

pub fn stellar_disk_integrated_single_scattering(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> SphericalScatteringResult {
    let samples = stellar_disk_sample_directions(
        geometric_sun_dir,
        star_angular_radius,
        disk_samples,
        limb_darkening_coeff,
    );

    let up = ray_origin.normalized();
    let refr = atmosphere.gas_optical_properties.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(
        refr,
        atmosphere.surface_temperature,
        atmosphere.surface_pressure,
    );

    let mut in_scatter_accum = 0.0;
    let mut opt_depth_accum = 0.0;
    let mut trans_accum = 0.0;

    for (s_dir, weight) in samples {
        let s_refracted = refracted_sun_direction(
            s_dir,
            up,
            refr_actual,
            atmosphere.gas_scale_height,
            atmosphere.planet_radius,
        );

        let res = single_scattering_spectral_radiance(
            ray_origin,
            ray_dir,
            s_refracted,
            solar_spectral_irradiance,
            wavelength,
            atmosphere,
            config,
        );

        in_scatter_accum += res.in_scattered_radiance * weight;
        opt_depth_accum += res.optical_depth * weight;
        trans_accum += res.transmittance * weight;
    }

    SphericalScatteringResult {
        in_scattered_radiance: in_scatter_accum,
        optical_depth: opt_depth_accum,
        transmittance: trans_accum,
    }
}

pub fn stellar_disk_direct_radiance_rgb(
    view_dir: Vector3,
    observer_pos: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    star_surface_radiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
    limb_darkening_coeff: f64,
) -> ColorRGB {
    let up = observer_pos.normalized();
    let refr = atmosphere.gas_optical_properties.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(
        refr,
        atmosphere.surface_temperature,
        atmosphere.surface_pressure,
    );

    let v_unrefracted = unrefracted_sun_direction(
        view_dir,
        up,
        refr_actual,
        atmosphere.gas_scale_height,
        atmosphere.planet_radius,
    );

    let s = geometric_sun_dir.normalized();
    let cos_angle = v_unrefracted.dot(&s).clamp(-1.0, 1.0);
    let angle_to_center = cos_angle.acos();
    let theta_max = star_angular_radius.value();

    if angle_to_center > theta_max || theta_max <= 0.0 {
        return ColorRGB::zero();
    }

    let rho = (angle_to_center / theta_max).clamp(0.0, 1.0);
    let mu = (1.0 - rho * rho).max(0.0).sqrt();
    let ld = stellar_limb_darkening(mu, limb_darkening_coeff);

    let w_r = Wavelength::new(680.0e-9);
    let w_g = Wavelength::new(550.0e-9);
    let w_b = Wavelength::new(440.0e-9);

    let trans_r = spherical_view_transmittance(observer_pos, view_dir, w_r, atmosphere, config);
    let trans_g = spherical_view_transmittance(observer_pos, view_dir, w_g, atmosphere, config);
    let trans_b = spherical_view_transmittance(observer_pos, view_dir, w_b, atmosphere, config);

    ColorRGB::new(
        star_surface_radiance_rgb.r() * ld * trans_r,
        star_surface_radiance_rgb.g() * ld * trans_g,
        star_surface_radiance_rgb.b() * ld * trans_b,
    )
}
