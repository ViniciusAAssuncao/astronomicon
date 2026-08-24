use crate::climate::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::geophysics::resolve_planetary_core;
use crate::gravity::resolve_entity_effective_mass;
use crate::hierarchy::resolve_parent_mass;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::shape::planet_mean_density;
use crate::tectonics::resolve_tectonic_setup;
use crate::tidal::resolve_tidal_diagnostics;
use astronomicon_core::domain::{OrbitalParent, Planet, PlanetRheology, TectonicRegime};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::hydrosphere::HydrosphereStructure;
use astronomicon_core::math::seismology::{
    equilibrium_tidal_bulge_height, radial_tidal_stress_amplitude,
    tectonic_seismic_energy_rate, tidal_seismic_energy_rate,
};
use astronomicon_core::math::tidal::fallback_love_number_k2;
use astronomicon_core::units::{Duration, Length, Luminosity, Mass, Pressure, Speed};
use astronomicon_db::repositories::{
    hydrosphere_repository, lithosphere_repository, planet_repository,
};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeismicDiagnostic {
    pub tectonic_regime: TectonicRegime,
    pub lithosphere_thickness: Length,
    pub plate_rms_velocity: Speed,
    pub tectonic_plate_count: u32,
    pub tectonic_seismic_energy: Luminosity,
    pub tidal_stress_amplitude: Pressure,
    pub tidal_bulge_height: Length,
    pub tidal_seismic_energy: Luminosity,
    pub total_seismic_energy: Luminosity,
}

pub async fn resolve_seismic_diagnostics(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<SeismicDiagnostic> {
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

    let mu_planet = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu_planet, radius);

    let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await?;
    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;

    let has_water = hydrosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .is_some();

    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let hydro_structure = hydro_diag.map(|h| HydrosphereStructure {
        total_volume_m3: 0.0,
        total_mass: h.total_mass,
        ice_thickness: h.ice_thickness,
        liquid_depth: h.liquid_depth,
        is_subsurface_ocean: h.is_subsurface_ocean,
        is_completely_frozen: h.is_completely_frozen,
        is_completely_liquid: h.is_completely_liquid,
    });

    let tectonic = resolve_tectonic_setup(
        &planet,
        &rheology,
        radius,
        g,
        surf_temp,
        &core_diag,
        has_water,
        hydro_structure.as_ref(),
    );

    let tectonic_energy = tectonic_seismic_energy_rate(
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

    let tidal_diag = resolve_tidal_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let (parent_mass, semi_major_axis, eccentricity) =
        match (planet.orbital_parent(), planet.orbital_elements()) {
            (OrbitalParent::Fixed, _) | (_, None) => (Mass::new(0.0), Length::new(0.0), 0.0),
            (parent, Some(elements)) => {
                let pm = if let Some(sys_id) = planet.star_system_id() {
                    let parent_id = match parent {
                        OrbitalParent::Star(id)
                        | OrbitalParent::Planet(id)
                        | OrbitalParent::Barycenter(id)
                        | OrbitalParent::MinorPlanet(id) => id,
                        OrbitalParent::Fixed => unreachable!(),
                    };
                    match resolve_entity_effective_mass(pool, &sys_id, &parent_id).await {
                        Ok(m) => m,
                        Err(_) => resolve_parent_mass(pool, &parent)
                            .await
                            .unwrap_or(Mass::new(0.0)),
                    }
                } else {
                    resolve_parent_mass(pool, &parent)
                        .await
                        .unwrap_or(Mass::new(0.0))
                };
                (pm, elements.semi_major_axis(), elements.eccentricity())
            }
        };

    let (h_tide, delta_sigma) =
        if parent_mass.value() > 0.0 && semi_major_axis.value() > 0.0 && eccentricity > 0.0 {
            let mean_rho = planet_mean_density(&planet);
            let k2 = planet
                .love_number_k2()
                .unwrap_or_else(|| fallback_love_number_k2(planet.kind(), Some(mean_rho)));

            let h_bulge = equilibrium_tidal_bulge_height(
                parent_mass,
                planet.mass(),
                radius,
                semi_major_axis,
                k2,
            );

            let crust_rho = rheology.mean_density();
            let d_sigma = radial_tidal_stress_amplitude(eccentricity, crust_rho, g, h_bulge);

            (h_bulge, d_sigma)
        } else {
            (Length::new(0.0), Pressure::new(0.0))
        };

    let tidal_energy = tidal_seismic_energy_rate(
        tidal_diag.tidal_heating_energy,
        radius,
        tectonic.z_brittle,
        tectonic.seismic_efficiency,
    );

    let total_energy = tectonic_energy + tidal_energy;

    Ok(SeismicDiagnostic {
        tectonic_regime: tectonic.regime,
        lithosphere_thickness: tectonic.z_lith,
        plate_rms_velocity: tectonic.plate_velocity,
        tectonic_plate_count: tectonic.plate_count,
        tectonic_seismic_energy: tectonic_energy,
        tidal_stress_amplitude: delta_sigma,
        tidal_bulge_height: h_tide,
        tidal_seismic_energy: tidal_energy,
        total_seismic_energy: total_energy,
    })
}