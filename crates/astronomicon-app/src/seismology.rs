use crate::climate::resolve_latitudinal_surface_temperature;
use crate::error::AppResult;
use crate::geophysics::resolve_planetary_core;
use crate::gravity::resolve_entity_effective_mass;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::shape::planet_mean_density;
use astronomicon_core::domain::{OrbitalParent, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::geology::{
    determine_tectonic_regime, lithosphere_thickness_for_planet,
    lithosphere_yield_strength, plate_rms_velocity, tectonic_plate_count,
};
use astronomicon_core::math::gravity::{
    combined_gravitational_parameter, gravitational_parameter, surface_gravity,
};
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::math::seismology::{
    equilibrium_tidal_bulge_height, radial_tidal_stress_amplitude,
    tectonic_seismic_energy_rate, tidal_seismic_energy_rate,
};
use astronomicon_core::math::tidal::fallback_love_number_k2;
use astronomicon_core::units::constants::{
    BASE_LITHOSPHERE_YIELD_STRESS, CRUST_DENSITY_REFERENCE, LITHOSPHERE_SHEAR_MODULUS,
    MANTLE_THERMAL_EXPANSION, SEISMIC_EFFICIENCY_FACTOR, SPECIFIC_HEAT_CAPACITY_ROCK,
};
use astronomicon_core::units::{
    Angle, Density, Duration, Length, Luminosity, Mass, Pressure, Speed,
};
use astronomicon_db::repositories::{hydrosphere_repository, planet_repository, star_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SeismicDiagnostic {
    pub tectonic_regime: astronomicon_core::domain::TectonicRegime,
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

    let z_lith = lithosphere_thickness_for_planet(
        planet.kind(),
        planet.mass(),
        surf_temp,
        core_diag.total_surface_heat_flux,
    );

    let has_water = hydrosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .is_some();

    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let hydro_structure = hydro_diag.map(|h| astronomicon_core::math::HydrosphereStructure {
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
    );

    let v_plate = plate_rms_velocity(core_diag.convective_heat_flux, regime);
    let plate_count = tectonic_plate_count(radius, z_lith, regime);

    let yield_strength = lithosphere_yield_strength(
        Pressure::new(BASE_LITHOSPHERE_YIELD_STRESS),
        has_water,
    );

    let tectonic_energy = tectonic_seismic_energy_rate(
        regime,
        v_plate,
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

    let (h_tide, delta_sigma, tidal_energy) = if parent_mass.value() > 0.0
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

        let crust_rho = Density::new(CRUST_DENSITY_REFERENCE);
        let d_sigma = radial_tidal_stress_amplitude(eccentricity, crust_rho, g, h_bulge);

        let mu_orb = combined_gravitational_parameter(planet.mass(), parent_mass);
        let p_orb = orbital_period(semi_major_axis, mu_orb).unwrap_or(Duration::new(86400.0));

        let energy = tidal_seismic_energy_rate(
            d_sigma,
            radius,
            z_lith,
            p_orb,
            LITHOSPHERE_SHEAR_MODULUS,
            SEISMIC_EFFICIENCY_FACTOR,
        );

        (h_bulge, d_sigma, energy)
    } else {
        (
            Length::new(0.0),
            Pressure::new(0.0),
            Luminosity::new(0.0),
        )
    };

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
