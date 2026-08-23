use crate::climate::{
    find_parent_star, resolve_global_mean_temperature, resolve_star_emission_profile,
    resolve_wind_profile_at_latitude,
};
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::volcanism::resolve_planetary_volcanism;
use astronomicon_core::chemistry::optics::mean_gas_optical_properties;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::aerosol::{
    airborne_dust_density, cloud_condensate_density, composite_aerosol_properties,
    derived_aerosol_scale_height, dust_threshold_surface_wind,
    mie_scattering_coefficient_at_wavelength, refractivity_at_temperature_pressure,
    volcanic_aerosol_density,
};
use astronomicon_core::math::atmospheric_scattering::SphericalAtmosphere;
use astronomicon_core::math::colorimetry::{
    cie_color_matching_functions, linear_to_srgb_gamma, reinhard_extended_tone_map,
    xyz_to_linear_srgb, ColorXYZ,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::optics::{
    absorption_coefficient, mie_scattering_coefficient, rayleigh_scattering_coefficient,
    refracted_sun_direction,
};
use astronomicon_core::math::radiation::planck_spectral_radiance;
use astronomicon_core::math::radiometry::{stellar_angular_radius, stellar_solid_angle};
use astronomicon_core::math::scattering::{
    multiple_scattering_spectral_radiance,
    multiple_scattering_stellar_disk_spectral_radiance,
    MultipleScatteringConfig,
};
use astronomicon_core::math::thermodynamics::MatterState;
use astronomicon_core::math::volcanism::VolcanicEruptionStyle;
use astronomicon_core::units::constants::{
    CIE_WAVELENGTH_MAX_M, CIE_WAVELENGTH_MIN_M, CIE_WAVELENGTH_STEP_M,
    OPTICAL_REFERENCE_WAVELENGTH,
};
use astronomicon_core::units::{Angle, ColorRGB, Duration, Length, Vector3, Wavelength};
use astronomicon_db::repositories::{atmosphere_repository, hydrosphere_repository, planet_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScatteringCoefficients {
    pub rayleigh_r: f64,
    pub rayleigh_g: f64,
    pub rayleigh_b: f64,
    pub mie_r: f64,
    pub mie_g: f64,
    pub mie_b: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyColorDiagnostic {
    pub zenith_color: ColorRGB,
    pub horizon_color: ColorRGB,
    pub sunset_color: ColorRGB,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyDiagnostic {
    pub scattering: ScatteringCoefficients,
    pub colors: SkyColorDiagnostic,
    pub total_optical_depth_r: f64,
    pub total_optical_depth_g: f64,
    pub total_optical_depth_b: f64,
}

pub async fn resolve_sky_diagnostics(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Option<SkyDiagnostic>> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atm_row = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let atm = match atm_row {
        Some(a) => a,
        None => return Ok(None),
    };

    let star = find_parent_star(pool, &planet).await?;
    let (_, star_temp, r_emit) =
        resolve_star_emission_profile(pool, &star, universe_epoch, at_epoch).await?;

    let sys_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;
    let positions = resolve_system_positions(pool, sys_id, universe_epoch + at_epoch).await?;
    let pos_p = positions
        .get(&planet_id)
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: "planet position could not be resolved".to_string(),
        })?;
    let pos_s = positions
        .get(&star.id())
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: "star position could not be resolved".to_string(),
        })?;
    let distance = (*pos_p - *pos_s).magnitude();

    let star_angular_radius = stellar_angular_radius(r_emit, distance);
    let solid_angle_sun = stellar_solid_angle(star_angular_radius).value();

    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let wind_diag = resolve_wind_profile_at_latitude(
        pool,
        planet_id,
        Angle::new(0.0),
        universe_epoch,
        at_epoch,
    )
    .await?;
    let volc_diag = resolve_planetary_volcanism(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let atm_dens = atm.density_at_surface(surf_temp)?;
    let scale_h = atm.scale_height(g, surf_temp)?;

    let v_thresh = dust_threshold_surface_wind(g, atm_dens);
    let dust_dens = airborne_dust_density(wind_diag.surface_wind_speed, v_thresh, atm_dens, g);

    let eruption_style = if volc_diag.is_cryovolcanic {
        VolcanicEruptionStyle::Cryovolcanic
    } else if volc_diag.explosive_fraction > volc_diag.effusive_fraction {
        VolcanicEruptionStyle::Explosive
    } else if volc_diag.global_magma_production_rate.value() > 0.0 {
        VolcanicEruptionStyle::Effusive
    } else {
        VolcanicEruptionStyle::Inactive
    };

    let volc_dens = volcanic_aerosol_density(
        volc_diag.outgassing_rate_sulfur,
        eruption_style,
        volc_diag.global_magma_production_rate,
        eq_radius,
        scale_h,
    );

    let matter_state = hydro_diag
        .map(|h| h.dominant_state)
        .unwrap_or(MatterState::Solid);
    let cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);
    let cloud_dens = cloud_condensate_density(
        matter_state,
        cov,
        surf_temp,
        atm.surface_pressure(),
    );

    let aerosol_props = composite_aerosol_properties(dust_dens, volc_dens, cloud_dens);
    let derived_aero_h = derived_aerosol_scale_height(g, scale_h, atm_dens);
    let aerosol_scale_h = if derived_aero_h.value() > 0.0 {
        derived_aero_h
    } else {
        Length::new(1500.0)
    };

    let comp: Vec<(String, f64)> = atm
        .composition()
        .iter()
        .map(|c| (c.formula().to_string(), c.percentage()))
        .collect();
    let opt_props = mean_gas_optical_properties(&comp)?;

    let atmosphere = SphericalAtmosphere::new(
        eq_radius,
        Length::new(100_000.0),
        atm.surface_pressure(),
        surf_temp,
        scale_h,
        aerosol_scale_h,
        opt_props.clone(),
        aerosol_props,
    );

    let ground_albedo = planet.bond_albedo().unwrap_or(0.15);
    let ms_config = MultipleScatteringConfig::new(
        32,
        16,
        Length::new(100_000.0),
        ground_albedo,
        1.0,
    );

    let ray_origin = Vector3::new(0.0, eq_radius.value(), 0.0);
    let up = ray_origin.normalized();
    let refr = opt_props.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(
        refr,
        surf_temp,
        atm.surface_pressure(),
    );

    let view_zenith = Vector3::new(0.0, 1.0, 0.0);
    let sun_dir_day = Vector3::new((PI / 4.0).sin(), (PI / 4.0).cos(), 0.0).normalized();
    let s_refracted_day = refracted_sun_direction(
        sun_dir_day,
        up,
        refr_actual,
        scale_h,
        eq_radius,
    );

    let view_horizon = Vector3::new(1.0, 0.0, 0.0);
    let view_sunset = Vector3::new(1.0, 0.0, 0.0);
    let sun_dir_sunset = Vector3::new(1.0, 0.0, 0.0);

    let step = CIE_WAVELENGTH_STEP_M;
    let mut xyz_zenith = ColorXYZ::zero();
    let mut xyz_horizon = ColorXYZ::zero();
    let mut xyz_sunset = ColorXYZ::zero();

    let mut lambda_m = CIE_WAVELENGTH_MIN_M;
    while lambda_m <= CIE_WAVELENGTH_MAX_M {
        let wavelength = Wavelength::new(lambda_m);
        let b_lambda = planck_spectral_radiance(wavelength, star_temp);
        let solar_irradiance = b_lambda * solid_angle_sun;
        let cmf = cie_color_matching_functions(wavelength);

        if solar_irradiance > 0.0 && solar_irradiance.is_finite() {
            let res_zenith = multiple_scattering_spectral_radiance(
                ray_origin,
                view_zenith,
                sun_dir_day,
                solar_irradiance,
                wavelength,
                &atmosphere,
                &ms_config,
            );
            xyz_zenith = xyz_zenith + cmf * res_zenith.total_radiance;

            let res_horizon = multiple_scattering_spectral_radiance(
                ray_origin,
                view_horizon,
                s_refracted_day,
                solar_irradiance,
                wavelength,
                &atmosphere,
                &ms_config,
            );
            xyz_horizon = xyz_horizon + cmf * res_horizon.total_radiance;

            let res_sunset = multiple_scattering_stellar_disk_spectral_radiance(
                ray_origin,
                view_sunset,
                sun_dir_sunset,
                star_angular_radius,
                solar_irradiance,
                wavelength,
                &atmosphere,
                &ms_config,
                16,
                0.6,
            );
            let sunset_radiance = res_sunset.total_radiance + b_lambda * res_sunset.transmittance;
            xyz_sunset = xyz_sunset + cmf * sunset_radiance;
        }

        lambda_m += step;
    }

    xyz_zenith = xyz_zenith * step;
    xyz_horizon = xyz_horizon * step;
    xyz_sunset = xyz_sunset * step;

    let exposure = if xyz_zenith.y() > 1e-12 {
        1.0 / xyz_zenith.y()
    } else {
        1.0
    };

    let process_color = |xyz: ColorXYZ| {
        let rgb = xyz_to_linear_srgb(xyz);
        let exposed = reinhard_extended_tone_map(rgb * exposure, 4.0);
        linear_to_srgb_gamma(exposed)
    };

    let colors = SkyColorDiagnostic {
        zenith_color: process_color(xyz_zenith),
        horizon_color: process_color(xyz_horizon),
        sunset_color: process_color(xyz_sunset),
    };

    let wl_r = Wavelength::new(680e-9);
    let wl_g = Wavelength::new(550e-9);
    let wl_b = Wavelength::new(440e-9);
    let ref_wl = Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH);

    let p_surf = atm.surface_pressure();
    let refr_stp = opt_props.refractivity_stp();
    let king = opt_props.king_factor();

    let b_r_r = rayleigh_scattering_coefficient(wl_r, refr_stp, king, p_surf, surf_temp);
    let b_r_g = rayleigh_scattering_coefficient(wl_g, refr_stp, king, p_surf, surf_temp);
    let b_r_b = rayleigh_scattering_coefficient(wl_b, refr_stp, king, p_surf, surf_temp);

    let b_m_r = mie_scattering_coefficient(&aerosol_props, wl_r, ref_wl);
    let b_m_g = mie_scattering_coefficient(&aerosol_props, wl_g, ref_wl);
    let b_m_b = mie_scattering_coefficient(&aerosol_props, wl_b, ref_wl);

    let b_a_r = absorption_coefficient(&opt_props, wl_r, p_surf, surf_temp);
    let b_a_g = absorption_coefficient(&opt_props, wl_g, p_surf, surf_temp);
    let b_a_b = absorption_coefficient(&opt_props, wl_b, p_surf, surf_temp);

    let b_em_r = mie_scattering_coefficient_at_wavelength(
        aerosol_props.base_extinction_coefficient(),
        wl_r,
        ref_wl,
        aerosol_props.angstrom_exponent(),
    )
    .max(b_m_r);
    let b_em_g = mie_scattering_coefficient_at_wavelength(
        aerosol_props.base_extinction_coefficient(),
        wl_g,
        ref_wl,
        aerosol_props.angstrom_exponent(),
    )
    .max(b_m_g);
    let b_em_b = mie_scattering_coefficient_at_wavelength(
        aerosol_props.base_extinction_coefficient(),
        wl_b,
        ref_wl,
        aerosol_props.angstrom_exponent(),
    )
    .max(b_m_b);

    let tau_tot_r = (b_r_r + b_a_r) * scale_h.value() + b_em_r * aerosol_scale_h.value();
    let tau_tot_g = (b_r_g + b_a_g) * scale_h.value() + b_em_g * aerosol_scale_h.value();
    let tau_tot_b = (b_r_b + b_a_b) * scale_h.value() + b_em_b * aerosol_scale_h.value();

    let scattering = ScatteringCoefficients {
        rayleigh_r: b_r_r,
        rayleigh_g: b_r_g,
        rayleigh_b: b_r_b,
        mie_r: b_m_r,
        mie_g: b_m_g,
        mie_b: b_m_b,
    };

    Ok(Some(SkyDiagnostic {
        scattering,
        colors,
        total_optical_depth_r: tau_tot_r,
        total_optical_depth_g: tau_tot_g,
        total_optical_depth_b: tau_tot_b,
    }))
}