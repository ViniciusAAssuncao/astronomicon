use crate::climate::find_parent_star;
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::geophysics::resolve_planetary_core;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::radiometry::equilibrium_temperature;
use astronomicon_core::math::thermodynamics::MatterState;
use astronomicon_core::units::{Duration, HeatFlux, Length, Mass, Pressure, Temperature};
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HydrosphereDiagnostic {
    pub dominant_state: MatterState,
    pub total_mass: Mass,
    pub surface_boiling_point: Temperature,
    pub surface_freezing_point: Temperature,
    pub ice_thickness: Length,
    pub liquid_depth: Length,
    pub is_subsurface_ocean: bool,
    pub is_completely_frozen: bool,
    pub is_completely_liquid: bool,
}

pub async fn resolve_hydrosphere_diagnostics(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Option<HydrosphereDiagnostic>> {
    let hydrosphere = match hydrosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(h) => h,
        None => return Ok(None),
    };

    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let radius = planet
        .equatorial_radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "equatorial_radius".to_string(),
            reason: format!("planet '{}' has no equatorial radius", planet_id),
        })?;

    let surface_pressure = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atm) => atm.surface_pressure(),
        None => Pressure::new(0.0),
    };

    let star = find_parent_star(pool, &planet).await?;

    let star_temp = star
        .effective_temperature()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "effective_temperature".to_string(),
            reason: "star does not have effective temperature".to_string(),
        })?;

    let star_radius = star
        .radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: "star does not have radius".to_string(),
        })?;

    let system_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let positions = resolve_system_positions(pool, system_id, total_epoch).await?;

    let planet_pos = positions
        .get(&planet.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("position for planet '{}' could not be resolved", planet.id()),
        })?;

    let star_pos = positions
        .get(&star.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("position for star '{}' could not be resolved", star.id()),
        })?;

    let orbital_distance = (planet_pos - star_pos).magnitude();
    let base_albedo = planet.bond_albedo().unwrap_or(0.3);

    let base_eq_temp =
        equilibrium_temperature(star_temp, star_radius, orbital_distance, base_albedo);
    let greenhouse = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atm) => atm.greenhouse_effect(),
        None => Temperature::new(0.0),
    };

    let base_temp = base_eq_temp + greenhouse;

    let initial_state = hydrosphere.matter_state(base_temp, surface_pressure)?;
    let dynamic_albedo = hydrosphere.dynamic_albedo(base_albedo, initial_state)?;

    let iterated_eq_temp =
        equilibrium_temperature(star_temp, star_radius, orbital_distance, dynamic_albedo);
    let surface_temp = iterated_eq_temp + greenhouse;

    let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await;
    let q_geo = match core_diag {
        Ok(core) => core.total_surface_heat_flux,
        Err(_) => HeatFlux::new(0.05),
    };

    let structure = hydrosphere.layer_structure(radius, surface_temp, q_geo)?;
    let final_state = hydrosphere.matter_state(surface_temp, surface_pressure)?;
    let t_boil = hydrosphere.boiling_point(surface_pressure)?;
    let t_freeze = hydrosphere.freezing_point()?;

    Ok(Some(HydrosphereDiagnostic {
        dominant_state: final_state,
        total_mass: structure.total_mass,
        surface_boiling_point: t_boil,
        surface_freezing_point: t_freeze,
        ice_thickness: structure.ice_thickness,
        liquid_depth: structure.liquid_depth,
        is_subsurface_ocean: structure.is_subsurface_ocean,
        is_completely_frozen: structure.is_completely_frozen,
        is_completely_liquid: structure.is_completely_liquid,
    }))
}
