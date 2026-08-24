pub mod color_processing;
pub mod profiles;
pub mod scattering_summary;
pub mod spectral_integration;

pub use color_processing::*;
pub use profiles::*;
pub use scattering_summary::*;
pub use spectral_integration::*;

use crate::climate::{
    resolve_global_mean_temperature, resolve_star_emission_profile,
    resolve_wind_profile_at_latitude,
};
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use crate::volcanism::resolve_planetary_volcanism;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::radiometry::{stellar_angular_radius, stellar_solid_angle};
use astronomicon_core::math::scattering::MultipleScatteringConfig;
use astronomicon_core::units::{Angle, ColorRGB, Duration, Length, Vector3};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use serde::{Deserialize, Serialize};
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

    let star = find_parent_star(pool, planet.orbital_parent()).await?;
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
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let ocean_cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);

    let (atmosphere, dust_profile, volcanic_profile, opt_props, scale_h) = build_sky_atmosphere(
        &planet,
        &atm,
        surf_temp,
        wind_diag.surface_wind_speed,
        &volc_diag,
        ocean_cov,
        eq_radius,
        g,
    )?;

    let ground_albedo = planet.bond_albedo().unwrap_or(0.15);
    let ms_config =
        MultipleScatteringConfig::new(32, 16, Length::new(100_000.0), ground_albedo, 1.0);

    let radiances = integrate_sky_spectrum(
        &atmosphere,
        &ms_config,
        star_temp,
        solid_angle_sun,
        star_angular_radius,
        eq_radius,
        scale_h,
        opt_props.refractivity_stp(),
        surf_temp,
        atm.surface_pressure(),
    );

    let colors = process_sky_colors(
        radiances.xyz_zenith,
        radiances.xyz_horizon,
        radiances.xyz_sunset,
        radiances.xyz_sun_toa,
    );

    let ray_origin = Vector3::new(0.0, eq_radius.value(), 0.0);
    let summary = resolve_scattering_summary(
        &atmosphere,
        &dust_profile,
        &volcanic_profile,
        &opt_props,
        atm.surface_pressure(),
        surf_temp,
        ray_origin,
    );

    Ok(Some(SkyDiagnostic {
        scattering: summary.scattering,
        colors,
        total_optical_depth_r: summary.total_optical_depth_r,
        total_optical_depth_g: summary.total_optical_depth_g,
        total_optical_depth_b: summary.total_optical_depth_b,
    }))
}
