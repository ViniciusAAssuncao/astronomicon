pub mod color;
pub mod exposure;
pub mod optical_column;
pub mod radiance;

pub use color::*;
pub use exposure::*;
pub use optical_column::*;
pub use radiance::*;

use crate::climate::clouds::cover::{CloudCoverDiagnostic, resolve_cloud_cover};
use crate::climate::emission::resolve_star_emission_profile;
use crate::climate::temperature::{
    resolve_global_mean_temperature, resolve_surface_albedo, resolve_top_of_atmosphere_irradiance,
};
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::units::{Duration, Length};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
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
pub struct SkyDiagnostic {
    pub scattering: ScatteringCoefficients,
    pub colors: SkyColorDiagnostic,
    pub total_optical_depth_r: f64,
    pub total_optical_depth_g: f64,
    pub total_optical_depth_b: f64,
    pub clouds: CloudCoverDiagnostic,
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
    let (_, star_temp, _) =
        resolve_star_emission_profile(pool, &star, universe_epoch, at_epoch).await?;

    let toa_irradiance =
        resolve_top_of_atmosphere_irradiance(pool, &planet, &star, universe_epoch, at_epoch)
            .await?;

    let optical_depth =
        resolve_optical_column(pool, planet_id, universe_epoch, at_epoch).await?;

    let surface_albedo =
        resolve_surface_albedo(pool, planet_id, universe_epoch, at_epoch).await?;

    let solar_irradiance = resolve_spectral_solar_irradiance(toa_irradiance, star_temp);
    let radiances = calculate_sky_radiances(&optical_depth, solar_irradiance, surface_albedo);
    let colors = process_sky_colors_from_radiances(&radiances);

    let cloud_diag = resolve_cloud_cover(pool, planet_id, universe_epoch, at_epoch).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let scale_h = atm.scale_height(g, surf_temp)?;
    let scale_h_val = scale_h.value().max(1.0);

    let scattering = ScatteringCoefficients {
        rayleigh_r: optical_depth.rayleigh_r / scale_h_val,
        rayleigh_g: optical_depth.rayleigh_g / scale_h_val,
        rayleigh_b: optical_depth.rayleigh_b / scale_h_val,
        mie_r: optical_depth.aerosol_r / scale_h_val,
        mie_g: optical_depth.aerosol_g / scale_h_val,
        mie_b: optical_depth.aerosol_b / scale_h_val,
    };

    Ok(Some(SkyDiagnostic {
        scattering,
        colors,
        total_optical_depth_r: optical_depth.total_r,
        total_optical_depth_g: optical_depth.total_g,
        total_optical_depth_b: optical_depth.total_b,
        clouds: cloud_diag,
    }))
}
