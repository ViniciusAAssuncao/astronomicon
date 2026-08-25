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
use astronomicon_core::math::radiometry::{photopic_illuminance, photopic_luminance};
use astronomicon_core::units::{Duration, Illuminance, Length, Luminance};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
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
pub struct SkyDiagnostic {
    pub scattering: ScatteringCoefficients,
    pub colors: SkyColorDiagnostic,
    pub human_eye_colors: SkyColorDiagnostic,
    pub total_optical_depth_r: f64,
    pub total_optical_depth_g: f64,
    pub total_optical_depth_b: f64,
    pub diffuse_radiance_r: f64,
    pub diffuse_radiance_g: f64,
    pub diffuse_radiance_b: f64,
    pub photopic_illuminance: Illuminance,
    pub zenith_luminance: Luminance,
    pub horizon_luminance: Luminance,
    pub sunset_luminance: Luminance,
    pub sunset_halo_luminance: Luminance,
    pub exposure_value: f64,
    pub human_eye_exposure_value: f64,
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
    let human_eye_colors = process_locally_adapted_sky_colors(&radiances);

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

    let dir_r = solar_irradiance.r * (-optical_depth.total_r).exp();
    let dir_g = solar_irradiance.g * (-optical_depth.total_g).exp();
    let dir_b = solar_irradiance.b * (-optical_depth.total_b).exp();
    let diff_irr_r = PI * radiances.zenith_diffuse.r;
    let diff_irr_g = PI * radiances.zenith_diffuse.g;
    let diff_irr_b = PI * radiances.zenith_diffuse.b;

    let tot_irr_r = dir_r + diff_irr_r;
    let tot_irr_g = dir_g + diff_irr_g;
    let tot_irr_b = dir_b + diff_irr_b;

    let phot_illum = photopic_illuminance(tot_irr_r, tot_irr_g, tot_irr_b);
    let z_lum = photopic_luminance(
        radiances.zenith_radiance.r,
        radiances.zenith_radiance.g,
        radiances.zenith_radiance.b,
    );
    let h_lum = photopic_luminance(
        radiances.horizon_radiance.r,
        radiances.horizon_radiance.g,
        radiances.horizon_radiance.b,
    );
    let s_lum = photopic_luminance(
        radiances.sunset_radiance.r,
        radiances.sunset_radiance.g,
        radiances.sunset_radiance.b,
    );
    let sh_lum = photopic_luminance(
        radiances.sunset_halo_radiance.r,
        radiances.sunset_halo_radiance.g,
        radiances.sunset_halo_radiance.b,
    );

    let ev = ev100_from_illuminance(phot_illum);
    let human_ev = ev100_from_luminance(z_lum);

    Ok(Some(SkyDiagnostic {
        scattering,
        colors,
        human_eye_colors,
        total_optical_depth_r: optical_depth.total_r,
        total_optical_depth_g: optical_depth.total_g,
        total_optical_depth_b: optical_depth.total_b,
        diffuse_radiance_r: radiances.zenith_diffuse.r,
        diffuse_radiance_g: radiances.zenith_diffuse.g,
        diffuse_radiance_b: radiances.zenith_diffuse.b,
        photopic_illuminance: phot_illum,
        zenith_luminance: z_lum,
        horizon_luminance: h_lum,
        sunset_luminance: s_lum,
        sunset_halo_luminance: sh_lum,
        exposure_value: ev,
        human_eye_exposure_value: human_ev,
        clouds: cloud_diag,
    }))
}