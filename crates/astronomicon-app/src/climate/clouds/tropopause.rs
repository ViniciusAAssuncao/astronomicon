use crate::climate::temperature::{
    resolve_advective_surface_temperature, resolve_global_mean_temperature,
};
use crate::error::AppResult;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::thermodynamics::{
    grey_atmosphere_skin_temperature, tropopause_altitude,
};
use astronomicon_core::units::{
    Acceleration, Angle, Duration, Length, Temperature, TemperatureGradient,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TropopauseDiagnostic {
    pub radiative_equilibrium_temperature: Temperature,
    pub skin_temperature: Temperature,
    pub surface_temperature: Temperature,
    pub tropopause_altitude: Length,
    pub tropopause_temperature: Temperature,
    pub lapse_rate: TemperatureGradient,
}

pub fn calculate_tropopause(
    surf_temp: Temperature,
    greenhouse: Temperature,
    env_lapse_rate: TemperatureGradient,
    gravity: Acceleration,
    atm_cp: f64,
) -> TropopauseDiagnostic {
    let t_eq = Temperature::new((surf_temp.value() - greenhouse.value()).max(0.0));
    let t_skin = grey_atmosphere_skin_temperature(t_eq);

    let dry_gamma = if env_lapse_rate.value() > 0.0 {
        env_lapse_rate
    } else {
        TemperatureGradient::new(gravity.value() / atm_cp.max(100.0))
    };

    let z_tropo = tropopause_altitude(surf_temp, t_skin, dry_gamma);

    TropopauseDiagnostic {
        radiative_equilibrium_temperature: t_eq,
        skin_temperature: t_skin,
        surface_temperature: surf_temp,
        tropopause_altitude: z_tropo,
        tropopause_temperature: t_skin,
        lapse_rate: dry_gamma,
    }
}

pub async fn resolve_tropopause(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<TropopauseDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;

    Ok(calculate_tropopause(
        surf_temp,
        atmosphere.greenhouse_effect(),
        atmosphere.lapse_rate(),
        g,
        atm_cp,
    ))
}

pub async fn resolve_tropopause_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<TropopauseDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let surf_temp = resolve_advective_surface_temperature(
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
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;

    Ok(calculate_tropopause(
        surf_temp,
        atmosphere.greenhouse_effect(),
        atmosphere.lapse_rate(),
        g,
        atm_cp,
    ))
}