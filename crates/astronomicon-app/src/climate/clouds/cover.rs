use crate::climate::atmosphere::{
    AtmosphericStratificationDiagnostic, resolve_atmospheric_stratification,
    resolve_atmospheric_stratification_at_latitude,
};
use crate::climate::clouds::instability::{
    ConvectiveInstabilityDiagnostic, resolve_convective_instability,
    resolve_convective_instability_at_latitude,
};
use crate::climate::clouds::layer::{CloudLayerDiagnostic, evaluate_cloud_layer};
use crate::climate::clouds::tropopause::{
    TropopauseDiagnostic, resolve_tropopause, resolve_tropopause_at_latitude,
};
use crate::climate::condensable_species::resolve_condensable_species;
use crate::error::AppResult;
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::domain::{Atmosphere, Planet};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::clouds::{
    CloudClassification, classify_cloud_system, cloud_band_altitudes,
    combine_layer_cloud_fractions_max_random_overlap, freezing_level_altitude,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::units::{Angle, Duration, Length, MolarMass, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudCoverDiagnostic {
    pub total_cloud_fraction: f64,
    pub low_cloud: CloudLayerDiagnostic,
    pub mid_cloud: CloudLayerDiagnostic,
    pub high_cloud: CloudLayerDiagnostic,
    pub freezing_level: Length,
    pub classification: CloudClassification,
}

pub fn calculate_cloud_cover(
    atmosphere: &Atmosphere,
    tropo: &TropopauseDiagnostic,
    instability: &ConvectiveInstabilityDiagnostic,
    stratification: &AtmosphericStratificationDiagnostic,
    solvent_props: &SolventProperties,
    solvent_molar_mass: MolarMass,
    surface_humidity: f64,
    freezing_point: Temperature,
    scale_h: Length,
) -> AppResult<CloudCoverDiagnostic> {
    let surf_temp = tropo.surface_temperature;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let freezing_level = freezing_level_altitude(surf_temp, freezing_point, env_lapse_rate)
        .unwrap_or(Length::new(0.0));

    let (z0, z_low_top, z_mid_top, z_high_top) = cloud_band_altitudes(tropo.tropopause_altitude);
    let z_low_mid = Length::new(0.5 * (z0.value() + z_low_top.value()));
    let z_mid_mid = Length::new(0.5 * (z_low_top.value() + z_mid_top.value()));
    let z_high_mid = Length::new(0.5 * (z_mid_top.value() + z_high_top.value()));

    let low_diag = evaluate_cloud_layer(
        atmosphere,
        z0,
        z_low_top,
        z_low_mid,
        surface_humidity,
        surf_temp,
        tropo.tropopause_altitude,
        tropo.skin_temperature,
        scale_h,
        atm_molar_mass,
        solvent_props,
        solvent_molar_mass,
        freezing_level,
    );

    let mid_diag = evaluate_cloud_layer(
        atmosphere,
        z_low_top,
        z_mid_top,
        z_mid_mid,
        surface_humidity,
        surf_temp,
        tropo.tropopause_altitude,
        tropo.skin_temperature,
        scale_h,
        atm_molar_mass,
        solvent_props,
        solvent_molar_mass,
        freezing_level,
    );

    let high_diag = evaluate_cloud_layer(
        atmosphere,
        z_mid_top,
        z_high_top,
        z_high_mid,
        surface_humidity,
        surf_temp,
        tropo.tropopause_altitude,
        tropo.skin_temperature,
        scale_h,
        atm_molar_mass,
        solvent_props,
        solvent_molar_mass,
        freezing_level,
    );

    let c_combined = combine_layer_cloud_fractions_max_random_overlap(
        low_diag.cloud_fraction,
        mid_diag.cloud_fraction,
        high_diag.cloud_fraction,
    );

    let total_cloud_fraction = atmosphere.cloud_coverage_fraction().unwrap_or(c_combined);

    let classification = classify_cloud_system(
        env_lapse_rate,
        instability.dry_adiabatic_lapse_rate,
        instability.moist_adiabatic_lapse_rate,
        instability.cape,
        stratification.cloud_top_altitude,
        tropo.tropopause_altitude,
        freezing_level,
    );

    Ok(CloudCoverDiagnostic {
        total_cloud_fraction,
        low_cloud: low_diag,
        mid_cloud: mid_diag,
        high_cloud: high_diag,
        freezing_level,
        classification,
    })
}

pub async fn resolve_cloud_cover(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<CloudCoverDiagnostic> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let (solvent_props, solvent_molar_mass, surface_humidity) =
        resolve_condensable_species(pool, planet_id).await?;
    let tropo = resolve_tropopause(pool, planet_id, universe_epoch, at_epoch).await?;
    let instability =
        resolve_convective_instability(pool, planet_id, universe_epoch, at_epoch).await?;
    let stratification =
        resolve_atmospheric_stratification(pool, planet_id, universe_epoch, at_epoch).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);
    let scale_h = atmosphere.scale_height(g, tropo.surface_temperature)?;

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let freezing_point = match &hydro_opt {
        Some(h) => h.freezing_point()?,
        None => solvent_props.normal_melting_point,
    };

    calculate_cloud_cover(
        &atmosphere,
        &tropo,
        &instability,
        &stratification,
        &solvent_props,
        solvent_molar_mass,
        surface_humidity,
        freezing_point,
        scale_h,
    )
}

pub async fn resolve_cloud_cover_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<CloudCoverDiagnostic> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let (solvent_props, solvent_molar_mass, surface_humidity) =
        resolve_condensable_species(pool, planet_id).await?;
    let tropo =
        resolve_tropopause_at_latitude(pool, planet_id, latitude, universe_epoch, at_epoch).await?;
    let instability = resolve_convective_instability_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;
    let stratification = resolve_atmospheric_stratification_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);
    let scale_h = atmosphere.scale_height(g, tropo.surface_temperature)?;

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let freezing_point = match &hydro_opt {
        Some(h) => h.freezing_point()?,
        None => solvent_props.normal_melting_point,
    };

    calculate_cloud_cover(
        &atmosphere,
        &tropo,
        &instability,
        &stratification,
        &solvent_props,
        solvent_molar_mass,
        surface_humidity,
        freezing_point,
        scale_h,
    )
}