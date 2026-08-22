use crate::climate::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::geophysics::resolve_planetary_core;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::tidal::resolve_tidal_diagnostics;
use astronomicon_core::domain::{Planet, TectonicRegime};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::geology::{
    determine_tectonic_regime, lithosphere_thickness_for_planet,
    lithosphere_yield_strength, plate_rms_velocity, tectonic_plate_count,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::hydrosphere::HydrosphereStructure;
use astronomicon_core::math::seismology::tectonic_seismic_energy_rate;
use astronomicon_core::units::constants::{
    BASE_LITHOSPHERE_YIELD_STRESS, MANTLE_THERMAL_EXPANSION,
    SEISMIC_EFFICIENCY_FACTOR, SPECIFIC_HEAT_CAPACITY_ROCK,
};
use astronomicon_core::units::{Duration, Length, Luminosity, Pressure, Speed};
use astronomicon_db::repositories::planet_repository;
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeologyDiagnostic {
    pub tectonic_regime: TectonicRegime,
    pub lithosphere_thickness: Length,
    pub plate_count: u32,
    pub plate_velocity: Speed,
    pub native_seismic_energy: Luminosity,
    pub tidal_seismic_energy: Luminosity,
}

pub async fn resolve_planetary_geology(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<GeologyDiagnostic> {
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

    let surface_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;
    let tidal_diag =
        resolve_tidal_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let mu_planet = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu_planet, radius);

    let z_lith = lithosphere_thickness_for_planet(
        planet.kind(),
        planet.mass(),
        surface_temp,
        core_diag.total_surface_heat_flux,
    );

    let (has_surface_liquid, hydro_structure) = match hydro_diag {
        Some(h) => (
            h.liquid_depth.value() > 0.0,
            Some(HydrosphereStructure {
                total_volume_m3: 0.0,
                total_mass: h.total_mass,
                ice_thickness: h.ice_thickness,
                liquid_depth: h.liquid_depth,
                is_subsurface_ocean: h.is_subsurface_ocean,
                is_completely_frozen: h.is_completely_frozen,
                is_completely_liquid: h.is_completely_liquid,
            }),
        ),
        None => (false, None),
    };

    let has_water_weakening = has_surface_liquid
        || planet.mantle_hydration_fraction().unwrap_or(0.0) > 0.001;

    let regime = determine_tectonic_regime(
        planet.kind(),
        radius,
        z_lith,
        g,
        core_diag.total_surface_heat_flux,
        core_diag.tidal_heat_flux,
        has_water_weakening,
        hydro_structure.as_ref(),
    );

    let plate_count = tectonic_plate_count(radius, z_lith, regime);
    let plate_vel = plate_rms_velocity(core_diag.convective_heat_flux, regime);

    let yield_strength = lithosphere_yield_strength(
        Pressure::new(BASE_LITHOSPHERE_YIELD_STRESS),
        has_water_weakening,
    );

    let native_seismic_energy = tectonic_seismic_energy_rate(
        regime,
        plate_vel,
        z_lith,
        radius,
        planet.mass(),
        core_diag.total_surface_heat_flux,
        yield_strength,
        plate_count,
        MANTLE_THERMAL_EXPANSION,
        SPECIFIC_HEAT_CAPACITY_ROCK,
        SEISMIC_EFFICIENCY_FACTOR,
    );

    let tidal_seismic_energy = Luminosity::new(tidal_diag.tidal_heating_energy.value() * SEISMIC_EFFICIENCY_FACTOR);

    Ok(GeologyDiagnostic {
        tectonic_regime: regime,
        lithosphere_thickness: z_lith,
        plate_count,
        plate_velocity: plate_vel,
        native_seismic_energy,
        tidal_seismic_energy,
    })
}
