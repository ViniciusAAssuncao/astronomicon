use crate::climate::resolve_latitudinal_surface_temperature;
use crate::error::AppResult;
use crate::geophysics::resolve_planetary_core;
use crate::gravity::resolve_entity_effective_mass;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::shape::planet_mean_density;
use crate::tidal::resolve_tidal_diagnostics;
use astronomicon_core::domain::{OrbitalParent, Planet, PlanetRheology, Star, TectonicRegime};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::geology::{
    brittle_ductile_transition_depth, determine_tectonic_regime, lithosphere_thickness,
    lithosphere_yield_strength, plate_rms_velocity, tectonic_plate_count,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::hydrosphere::HydrosphereStructure;
use astronomicon_core::math::seismology::{
    equilibrium_tidal_bulge_height, radial_tidal_stress_amplitude, seismic_efficiency,
    tectonic_seismic_energy_rate, tidal_seismic_energy_rate,
};
use astronomicon_core::math::tidal::fallback_love_number_k2;
use astronomicon_core::units::{
    Angle, Duration, Length, Luminosity, Mass, Pressure, Speed,
};
use astronomicon_db::repositories::{
    hydrosphere_repository, lithosphere_repository, planet_repository, star_repository,
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

async fn resolve_direct_parent_mass(pool: &SqlitePool, parent: &OrbitalParent) -> AppResult<Mass> {
    match parent {
        OrbitalParent::Fixed => Ok(Mass::new(0.0)),
        OrbitalParent::Star(star_id) => {
            let row = star_repository::get_by_id(pool, star_id)
                .await?
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "parent_star_id".to_string(),
                    reason: format!("star '{}' not found", star_id),
                })?;
            let star = Star::try_from(row)?;
            Ok(star.mass())
        }
        OrbitalParent::Planet(planet_id) => {
            let row = planet_repository::get_by_id(pool, planet_id)
                .await?
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "parent_planet_id".to_string(),
                    reason: format!("planet '{}' not found", planet_id),
                })?;
            let parent_planet = Planet::try_from(row)?;
            Ok(parent_planet.mass())
        }
        OrbitalParent::Barycenter(bary_id) => {
            let mut visited = std::collections::HashSet::new();
            let stars =
                crate::climate::collect_stars_from_barycenter(pool, bary_id, &mut visited).await?;
            let total_mass: f64 = stars.iter().map(|s| s.mass().value()).sum();
            Ok(Mass::new(total_mass))
        }
    }
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
    let surf_temp = resolve_latitudinal_surface_temperature(
        pool,
        planet_id,
        Angle::new(0.0),
        universe_epoch,
        at_epoch,
    )
    .await?;

    let z_lith = lithosphere_thickness(
        rheology.mean_solidus_temperature(),
        surf_temp,
        core_diag.total_surface_heat_flux,
        rheology.mean_thermal_conductivity(),
    );

    let z_brittle = brittle_ductile_transition_depth(
        z_lith,
        surf_temp,
        rheology.mean_solidus_temperature(),
        rheology.mean_solidus_temperature(),
    );

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

    let regime = determine_tectonic_regime(
        planet.kind(),
        radius,
        z_lith,
        g,
        core_diag.total_surface_heat_flux,
        core_diag.tidal_heat_flux,
        has_water,
        hydro_structure.as_ref(),
        &rheology,
    );

    let v_plate = plate_rms_velocity(core_diag.convective_heat_flux, regime);
    let plate_count = tectonic_plate_count(radius, z_lith, regime);

    let yield_strength = lithosphere_yield_strength(
        rheology.mean_base_yield_stress(),
        has_water,
    );

    let tectonic_energy = tectonic_seismic_energy_rate(
        regime,
        v_plate,
        z_brittle,
        radius,
        planet.mass(),
        core_diag.total_surface_heat_flux,
        yield_strength,
        rheology.mean_shear_modulus(),
        plate_count,
        rheology.mean_thermal_expansion(),
        rheology.mean_specific_heat_capacity(),
    );

    let tidal_diag =
        resolve_tidal_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let (parent_mass, semi_major_axis, eccentricity) = match (
        planet.orbital_parent(),
        planet.orbital_elements(),
    ) {
        (OrbitalParent::Fixed, _) | (_, None) => (Mass::new(0.0), Length::new(0.0), 0.0),
        (parent, Some(elements)) => {
            let pm = if let Some(sys_id) = planet.star_system_id() {
                let parent_id = match parent {
                    OrbitalParent::Star(id)
                    | OrbitalParent::Planet(id)
                    | OrbitalParent::Barycenter(id) => id,
                    OrbitalParent::Fixed => unreachable!(),
                };
                match resolve_entity_effective_mass(pool, &sys_id, &parent_id).await {
                    Ok(m) => m,
                    Err(_) => resolve_direct_parent_mass(pool, &parent)
                        .await
                        .unwrap_or(Mass::new(0.0)),
                }
            } else {
                resolve_direct_parent_mass(pool, &parent)
                    .await
                    .unwrap_or(Mass::new(0.0))
            };
            (pm, elements.semi_major_axis(), elements.eccentricity())
        }
    };

    let (h_tide, delta_sigma) = if parent_mass.value() > 0.0
        && semi_major_axis.value() > 0.0
        && eccentricity > 0.0
    {
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
        (
            Length::new(0.0),
            Pressure::new(0.0),
        )
    };

    let seismic_eff = seismic_efficiency(yield_strength, rheology.mean_shear_modulus());
    let tidal_energy = tidal_seismic_energy_rate(
        tidal_diag.tidal_heating_energy,
        radius,
        z_brittle,
        seismic_eff,
    );

    let total_energy = tectonic_energy + tidal_energy;

    Ok(SeismicDiagnostic {
        tectonic_regime: regime,
        lithosphere_thickness: z_lith,
        plate_rms_velocity: v_plate,
        tectonic_plate_count: plate_count,
        tectonic_seismic_energy: tectonic_energy,
        tidal_stress_amplitude: delta_sigma,
        tidal_bulge_height: h_tide,
        tidal_seismic_energy: tidal_energy,
        total_seismic_energy: total_energy,
    })
}