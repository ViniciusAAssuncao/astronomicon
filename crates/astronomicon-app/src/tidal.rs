use crate::error::AppResult;
use crate::gravity::resolve_entity_effective_mass;
use crate::shape::planet_mean_density;
use astronomicon_core::domain::{OrbitalParent, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::tidal::{
    fallback_love_number_k2, fallback_tidal_dissipation_factor_q, tidal_heating_surface_flux,
    tidal_heating_total_power, tidal_locking_timescale,
};
use astronomicon_core::units::{Duration, HeatFlux, Length, Luminosity, Mass};
use astronomicon_db::repositories::{planet_repository, star_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TidalDiagnostic {
    pub love_number_k2: f64,
    pub dissipation_factor_q: f64,
    pub tidal_heating_energy: Luminosity,
    pub tidal_surface_heat_flux: HeatFlux,
    pub tidal_locking_timescale: Duration,
    pub is_tidally_locked: bool,
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
            let stars = crate::climate::collect_stars_from_barycenter(pool, bary_id, &mut visited).await?;
            let total_mass: f64 = stars.iter().map(|s| s.mass().value()).sum();
            Ok(Mass::new(total_mass))
        }
    }
}

pub async fn resolve_tidal_diagnostics(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<TidalDiagnostic> {
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

    let mean_rho = planet_mean_density(&planet);
    let k2 = planet
        .love_number_k2()
        .unwrap_or_else(|| fallback_love_number_k2(planet.kind(), Some(mean_rho)));
    let q = planet
        .tidal_dissipation_factor_q()
        .unwrap_or_else(|| fallback_tidal_dissipation_factor_q(planet.kind()));

    let rot_period = planet
        .rotation_period()
        .unwrap_or_else(|| Duration::new(86400.0));

    let (parent_mass, semi_major_axis, eccentricity) = match (
        planet.orbital_parent(),
        planet.orbital_elements(),
    ) {
        (OrbitalParent::Fixed, _) | (_, None) => (Mass::new(0.0), Length::new(0.0), 0.0),
        (parent, Some(elements)) => {
            let pm = if let Some(sys_id) = planet.star_system_id() {
                let parent_id = match parent {
                    OrbitalParent::Star(id) | OrbitalParent::Planet(id) | OrbitalParent::Barycenter(id) => id,
                    OrbitalParent::Fixed => unreachable!(),
                };
                match resolve_entity_effective_mass(pool, &sys_id, &parent_id).await {
                    Ok(m) => m,
                    Err(_) => resolve_direct_parent_mass(pool, &parent).await.unwrap_or(Mass::new(0.0)),
                }
            } else {
                resolve_direct_parent_mass(pool, &parent).await.unwrap_or(Mass::new(0.0))
            };
            (pm, elements.semi_major_axis(), elements.eccentricity())
        }
    };

    let timescale = if parent_mass.value() > 0.0 && semi_major_axis.value() > 0.0 {
        tidal_locking_timescale(
            planet.mass(),
            radius,
            rot_period,
            semi_major_axis,
            parent_mass,
            k2,
            q,
        )
    } else {
        Duration::new(0.0)
    };

    let current_age = universe_epoch + at_epoch;
    let is_tidally_locked = timescale.value() > 0.0 && current_age.value() >= timescale.value();

    let (tidal_energy, tidal_flux) = if parent_mass.value() <= 0.0 || semi_major_axis.value() <= 0.0 {
        (Luminosity::new(0.0), HeatFlux::new(0.0))
    } else if is_tidally_locked && eccentricity <= 1e-12 {
        (Luminosity::new(0.0), HeatFlux::new(0.0))
    } else {
        let power = tidal_heating_total_power(
            parent_mass,
            planet.mass(),
            semi_major_axis,
            eccentricity,
            radius,
            k2,
            q,
        );
        let flux = tidal_heating_surface_flux(
            parent_mass,
            planet.mass(),
            semi_major_axis,
            eccentricity,
            radius,
            k2,
            q,
        );
        (power, flux)
    };

    Ok(TidalDiagnostic {
        love_number_k2: k2,
        dissipation_factor_q: q,
        tidal_heating_energy: tidal_energy,
        tidal_surface_heat_flux: tidal_flux,
        tidal_locking_timescale: timescale,
        is_tidally_locked,
    })
}
