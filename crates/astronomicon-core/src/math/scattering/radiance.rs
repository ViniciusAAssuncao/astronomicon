use crate::chemistry::optics::refractivity_at_temperature_pressure;
use crate::math::atmospheric_scattering::{
    SphericalAtmosphere, ray_atmosphere_segment, sun_path_optical_depth,
};
use crate::math::optics::{
    absorption_coefficient, henyey_greenstein_phase_function,
    rayleigh_phase_function_with_depolarization, rayleigh_scattering_coefficient,
    refracted_sun_direction,
};
use crate::math::radiometry::stellar_disk_sample_directions;
use crate::math::scattering::config::{MultipleScatteringConfig, MultipleScatteringResult};
use crate::math::scattering::ground_terms::{
    ground_reflected_radiance, ground_solid_angle_factor, isotropic_multiple_scattering_source,
    single_scattering_albedo,
};
use crate::units::{Angle, Length, Vector3, Wavelength};
use std::f64::consts::PI;

pub fn multiple_scattering_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig,
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
        atmosphere.atmosphere_top_radius,
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

    let r_p = atmosphere.planet_radius.value();
    let r_top = atmosphere.atmosphere_top_radius.value();
    let h_gas = atmosphere.gas_scale_height.value().max(1.0);

    let subsolar_ground_pos = s_dir * r_p;
    let ground_sun_depth =
        sun_path_optical_depth(subsolar_ground_pos, s_dir, atmosphere, config.sun_samples);

    let ground_incident_irradiance = match ground_sun_depth {
        Some(sd) => {
            let tau_g = sd.total_extinction_optical_depth(wavelength, atmosphere);
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
    let mut tau_view_dust = 0.0;
    let mut tau_view_cloud = 0.0;
    let mut tau_view_volc = 0.0;
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

        let b_e_d = rho_d * k_e_dust;
        let b_e_c = rho_c * k_e_cloud;
        let b_e_v = rho_v * k_e_volc;
        let b_e_aero = b_e_d + b_e_c + b_e_v;

        let g_eff = if b_s_aero > 0.0 {
            (b_s_d * g_dust + b_s_c * g_cloud + b_s_v * g_volc) / b_s_aero
        } else {
            0.0
        };

        let phase_m = henyey_greenstein_phase_function(theta, g_eff);

        let beta_s_loc = beta_s_r0 * rho_g + b_s_aero;
        let beta_e_loc = beta_e_r0 * rho_g + b_e_aero;
        let ssa_loc = single_scattering_albedo(beta_s_loc, beta_e_loc);

        let current_view_tau = beta_e_r0 * tau_view_gas
            + k_e_dust * tau_view_dust
            + k_e_cloud * tau_view_cloud
            + k_e_volc * tau_view_volc;

        let view_transmittance = if current_view_tau > 700.0 {
            0.0
        } else {
            (-current_view_tau).exp()
        };

        let sun_depth = sun_path_optical_depth(pos_i, s_dir, atmosphere, config.sun_samples);

        let (dir_irradiance_at_pos, sun_transmittance) = match sun_depth {
            Some(sun_tau) => {
                let tau_sun = sun_tau.total_extinction_optical_depth(wavelength, atmosphere);
                let trans = if tau_sun > 700.0 {
                    0.0
                } else {
                    (-tau_sun).exp()
                };
                (solar_spectral_irradiance * trans, trans)
            }
            None => (0.0, 0.0),
        };

        if sun_transmittance > 0.0 {
            let scatter_coeff = beta_s_r0 * rho_g * phase_r + b_s_aero * phase_m;
            in_scatter_ss += solar_spectral_irradiance
                * sun_transmittance
                * scatter_coeff
                * view_transmittance
                * ds;
        }

        let w_ground = ground_solid_angle_factor(Length::new(alt_i), atmosphere.planet_radius);
        let tau_g_to_z = beta_e_r0 * h_gas * (1.0 - rho_g)
            + k_e_dust
                * atmosphere
                    .dust_profile
                    .integrated_column_between(Length::new(0.0), alt_len)
            + k_e_cloud
                * atmosphere
                    .cloud_profile
                    .integrated_column_between(Length::new(0.0), alt_len)
            + k_e_volc
                * atmosphere
                    .volcanic_profile
                    .density_at_altitude(alt_len)
                    .value()
                * alt_i.min(atmosphere.volcanic_profile.plume_thickness.value());

        let trans_g_to_z = if tau_g_to_z > 700.0 {
            0.0
        } else {
            (-tau_g_to_z).exp()
        };
        let ground_irradiance_at_pos = l_ground * 2.0 * PI * w_ground * trans_g_to_z;

        let tau_top = beta_e_r0 * h_gas * rho_g
            + k_e_dust
                * atmosphere
                    .dust_profile
                    .integrated_column_between(alt_len, Length::new(r_top - r_p))
            + k_e_cloud
                * atmosphere
                    .cloud_profile
                    .integrated_column_between(alt_len, Length::new(r_top - r_p))
            + k_e_volc
                * atmosphere
                    .volcanic_profile
                    .density_at_altitude(alt_len)
                    .value()
                * (r_top - r_p - alt_i)
                    .max(0.0)
                    .min(atmosphere.volcanic_profile.plume_thickness.value());

        let j_ms = isotropic_multiple_scattering_source(
            dir_irradiance_at_pos,
            ground_irradiance_at_pos,
            ssa_loc,
            tau_top,
            config.multiple_scattering_factor,
        );

        in_scatter_ms += j_ms * beta_s_loc * view_transmittance * ds;
    }

    let ray_total_tau = beta_e_r0 * tau_view_gas
        + k_e_dust * tau_view_dust
        + k_e_cloud * tau_view_cloud
        + k_e_volc * tau_view_volc;

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

pub fn multiple_scattering_stellar_disk_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> MultipleScatteringResult {
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

    let mut ss_accum = 0.0;
    let mut ms_accum = 0.0;
    let mut total_accum = 0.0;
    let mut tau_accum = 0.0;
    let mut trans_accum = 0.0;

    for (s_dir, weight) in samples {
        let s_refracted = refracted_sun_direction(
            s_dir,
            up,
            refr_actual,
            atmosphere.gas_scale_height,
            atmosphere.planet_radius,
        );

        let res = multiple_scattering_spectral_radiance(
            ray_origin,
            ray_dir,
            s_refracted,
            solar_spectral_irradiance,
            wavelength,
            atmosphere,
            config,
        );

        ss_accum += res.single_scattered_radiance * weight;
        ms_accum += res.multiple_scattered_radiance * weight;
        total_accum += res.total_radiance * weight;
        tau_accum += res.optical_depth * weight;
        trans_accum += res.transmittance * weight;
    }

    MultipleScatteringResult {
        single_scattered_radiance: ss_accum,
        multiple_scattered_radiance: ms_accum,
        total_radiance: total_accum,
        optical_depth: tau_accum,
        transmittance: trans_accum,
    }
}
