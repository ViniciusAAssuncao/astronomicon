use crate::climate::atmosphere::{
    AtmosphericStratificationDiagnostic, resolve_atmospheric_stratification,
    resolve_atmospheric_stratification_at_latitude,
};
use crate::climate::clouds::tropopause::{
    TropopauseDiagnostic, resolve_tropopause, resolve_tropopause_at_latitude,
};
use crate::climate::condensable_species::resolve_condensable_species;
use crate::error::AppResult;
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::domain::{Atmosphere, Planet};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::clouds::{
    AtmosphericStability, CloudMorphology, classify_atmospheric_stability,
    classify_cloud_morphology, is_deep_convective_cloud,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::thermodynamics::{
    convective_available_potential_energy, convective_inhibition, dry_adiabatic_lapse_rate,
    equilibrium_level, integrate_parcel_buoyancy_profile, level_of_free_convection,
    moist_adiabatic_lapse_rate,
};
use astronomicon_core::units::{
    Acceleration, Angle, Duration, Length, MolarMass, SpecificEnergy, Temperature,
    TemperatureGradient,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConvectiveInstabilityDiagnostic {
    pub surface_dew_point: Temperature,
    pub lcl_altitude: Length,
    pub lfc_altitude: Option<Length>,
    pub equilibrium_level: Option<Length>,
    pub cape: SpecificEnergy,
    pub cin: SpecificEnergy,
    pub stability: AtmosphericStability,
    pub morphology: CloudMorphology,
    pub is_deep_convection: bool,
    pub dry_adiabatic_lapse_rate: TemperatureGradient,
    pub moist_adiabatic_lapse_rate: TemperatureGradient,
}

pub fn calculate_convective_instability(
    atmosphere: &Atmosphere,
    tropo: &TropopauseDiagnostic,
    stratification: &AtmosphericStratificationDiagnostic,
    solvent_props: &SolventProperties,
    solvent_molar_mass: MolarMass,
    gravity: Acceleration,
) -> AppResult<ConvectiveInstabilityDiagnostic> {
    let surf_temp = tropo.surface_temperature;
    let surf_press = atmosphere.surface_pressure();
    let scale_h = atmosphere.scale_height(gravity, surf_temp)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;
    let dew_point = stratification.surface_dew_point;

    let dry_gamma = dry_adiabatic_lapse_rate(gravity, atm_cp);
    let moist_gamma = moist_adiabatic_lapse_rate(
        gravity,
        atm_cp,
        surf_temp,
        surf_press,
        solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    let profile = integrate_parcel_buoyancy_profile(
        surf_temp,
        surf_press,
        dew_point,
        atmosphere.lapse_rate(),
        scale_h,
        gravity,
        atm_cp,
        tropo.tropopause_altitude,
        tropo.skin_temperature,
        solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    let lfc = level_of_free_convection(&profile, stratification.lcl_altitude);
    let el = match lfc {
        Some(z_lfc) => equilibrium_level(&profile, z_lfc),
        None => None,
    };

    let cape = match (lfc, el) {
        (Some(z_lfc), Some(z_el)) => convective_available_potential_energy(&profile, z_lfc, z_el),
        _ => SpecificEnergy::new(0.0),
    };

    let cin = match lfc {
        Some(z_lfc) => convective_inhibition(&profile, z_lfc),
        None => SpecificEnergy::new(0.0),
    };

    let stability =
        classify_atmospheric_stability(atmosphere.lapse_rate(), dry_gamma, moist_gamma);
    let morphology = classify_cloud_morphology(stability, cape);
    let is_deep = is_deep_convective_cloud(
        morphology,
        stratification.cloud_top_altitude,
        tropo.tropopause_altitude,
        cape,
    );

    Ok(ConvectiveInstabilityDiagnostic {
        surface_dew_point: dew_point,
        lcl_altitude: stratification.lcl_altitude,
        lfc_altitude: lfc,
        equilibrium_level: el,
        cape,
        cin,
        stability,
        morphology,
        is_deep_convection: is_deep,
        dry_adiabatic_lapse_rate: dry_gamma,
        moist_adiabatic_lapse_rate: moist_gamma,
    })
}

pub async fn resolve_convective_instability(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<ConvectiveInstabilityDiagnostic> {
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

    let tropo = resolve_tropopause(pool, planet_id, universe_epoch, at_epoch).await?;
    let stratification =
        resolve_atmospheric_stratification(pool, planet_id, universe_epoch, at_epoch).await?;
    let (solvent_props, solvent_molar_mass, _) =
        resolve_condensable_species(pool, planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    calculate_convective_instability(
        &atmosphere,
        &tropo,
        &stratification,
        &solvent_props,
        solvent_molar_mass,
        g,
    )
}

pub async fn resolve_convective_instability_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<ConvectiveInstabilityDiagnostic> {
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

    let tropo =
        resolve_tropopause_at_latitude(pool, planet_id, latitude, universe_epoch, at_epoch)
            .await?;
    let stratification = resolve_atmospheric_stratification_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;
    let (solvent_props, solvent_molar_mass, _) =
        resolve_condensable_species(pool, planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    calculate_convective_instability(
        &atmosphere,
        &tropo,
        &stratification,
        &solvent_props,
        solvent_molar_mass,
        g,
    )
}