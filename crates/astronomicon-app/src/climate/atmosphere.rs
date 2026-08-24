use crate::climate::condensable_species::resolve_condensable_species;
use crate::climate::temperature::resolve_global_mean_temperature;
use crate::error::AppResult;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::climate::temperature_at_altitude;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::thermodynamics::{
    cloud_top_altitude, dew_point_temperature, grey_atmosphere_skin_temperature,
    lifting_condensation_level, moist_adiabatic_lapse_rate, tropopause_altitude,
};
use astronomicon_core::units::{
    Density, Duration, Length, Pressure, Temperature, TemperatureGradient,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericStratificationDiagnostic {
    pub surface_dew_point: Temperature,
    pub lcl_altitude: Length,
    pub cloud_top_altitude: Length,
    pub moist_adiabatic_lapse_rate: TemperatureGradient,
}

pub async fn resolve_atmospheric_profile_at_altitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    surface_temperature: Temperature,
    altitude: Length,
) -> AppResult<(Pressure, Temperature, Density)> {
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

    let eq_radius = planet
        .equatorial_radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "equatorial_radius".to_string(),
            reason: "planet does not have equatorial radius".to_string(),
        })?;

    let mu = gravitational_parameter(planet.mass());
    let gravity = surface_gravity(mu, eq_radius);

    let temp_at_alt =
        temperature_at_altitude(surface_temperature, altitude, atmosphere.lapse_rate());
    let scale_h = atmosphere.scale_height(gravity, surface_temperature)?;
    let press_at_alt = atmosphere.pressure_at_altitude(altitude, scale_h);
    let molar_mass = atmosphere.mean_molar_mass()?;
    let density_at_alt = ideal_gas_density(press_at_alt, molar_mass, temp_at_alt);

    Ok((press_at_alt, temp_at_alt, density_at_alt))
}

pub async fn resolve_atmospheric_stratification(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<AtmosphericStratificationDiagnostic> {
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

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let surf_press = atmosphere.surface_pressure();
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let (solvent_props, solvent_molar_mass, humidity) =
        resolve_condensable_species(pool, planet_id).await?;

    let dew_point =
        dew_point_temperature(surf_temp, humidity, solvent_props.enthalpy_of_vaporization);
    let moist_gamma = moist_adiabatic_lapse_rate(
        g,
        atm_cp,
        surf_temp,
        surf_press,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    let dry_gamma = if env_lapse_rate.value() > 0.0 {
        env_lapse_rate
    } else {
        TemperatureGradient::new(g.value() / atm_cp.max(100.0))
    };

    let lcl = lifting_condensation_level(
        surf_temp,
        dew_point,
        dry_gamma,
        scale_h,
        solvent_props.enthalpy_of_vaporization,
    );

    let t_skin = grey_atmosphere_skin_temperature(surf_temp);
    let z_tropo = tropopause_altitude(surf_temp, t_skin, dry_gamma);

    let cloud_top = cloud_top_altitude(
        lcl,
        surf_temp,
        surf_press,
        dry_gamma,
        scale_h,
        g,
        atm_cp,
        z_tropo,
        t_skin,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    Ok(AtmosphericStratificationDiagnostic {
        surface_dew_point: dew_point,
        lcl_altitude: lcl,
        cloud_top_altitude: cloud_top,
        moist_adiabatic_lapse_rate: moist_gamma,
    })
}
