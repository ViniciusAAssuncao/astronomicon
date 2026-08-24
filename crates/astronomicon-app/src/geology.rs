use crate::climate::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::geophysics::resolve_planetary_core;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::tectonics::resolve_tectonic_setup;
use crate::tidal::resolve_tidal_diagnostics;
use astronomicon_core::domain::{Planet, PlanetRheology, TectonicRegime};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::hydrosphere::HydrosphereStructure;
use astronomicon_core::math::seismology::{
    tectonic_seismic_energy_rate, tidal_seismic_energy_rate,
};
use astronomicon_core::units::{Duration, Length, Luminosity, Speed};
use astronomicon_db::repositories::{lithosphere_repository, planet_repository};
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

    let rheology = match lithosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(r) => r,
        None => PlanetRheology::fallback_for_kind(planet.kind()),
    };

    let surface_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;
    let tidal_diag =
        resolve_tidal_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let mu_planet = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu_planet, radius);

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

    let tectonic = resolve_tectonic_setup(
        &planet,
        &rheology,
        radius,
        g,
        surface_temp,
        &core_diag,
        has_water_weakening,
        hydro_structure.as_ref(),
    );

    let native_seismic_energy = tectonic_seismic_energy_rate(
        tectonic.regime,
        tectonic.plate_velocity,
        tectonic.z_brittle,
        radius,
        planet.mass(),
        core_diag.total_surface_heat_flux,
        tectonic.yield_strength,
        rheology.mean_shear_modulus(),
        tectonic.plate_count,
        rheology.mean_thermal_expansion(),
        rheology.mean_specific_heat_capacity(),
    );

    let tidal_seismic_energy = tidal_seismic_energy_rate(
        tidal_diag.tidal_heating_energy,
        radius,
        tectonic.z_brittle,
        tectonic.seismic_efficiency,
    );

    Ok(GeologyDiagnostic {
        tectonic_regime: tectonic.regime,
        lithosphere_thickness: tectonic.z_lith,
        plate_count: tectonic.plate_count,
        plate_velocity: tectonic.plate_velocity,
        native_seismic_energy,
        tidal_seismic_energy,
    })
}
