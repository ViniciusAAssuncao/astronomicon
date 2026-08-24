use crate::error::AppResult;
use crate::gravity::resolve_entity_effective_mass;
use crate::hierarchy::resolve_parent_mass;
use crate::shape::planet_mean_density;
use astronomicon_core::domain::{OrbitalParent, Planet};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::geophysics::{
    conducting_core_radius, convective_core_heat_flux, core_density,
    core_mantle_boundary_heat_flux, radiogenic_heat_flux, total_surface_geothermal_heat_flux,
};
use astronomicon_core::math::tidal::{
    fallback_love_number_k2, fallback_tidal_dissipation_factor_q, tidal_heating_surface_flux,
};
use astronomicon_core::units::{Density, Duration, HeatFlux, Length, Mass};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryCoreDiagnostic {
    pub core_radius: Length,
    pub core_density: Density,
    pub cmb_heat_flux: HeatFlux,
    pub convective_heat_flux: HeatFlux,
    pub radiogenic_heat_flux: HeatFlux,
    pub tidal_heat_flux: HeatFlux,
    pub total_surface_heat_flux: HeatFlux,
}

pub async fn resolve_planetary_core(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<PlanetaryCoreDiagnostic> {
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

    let cmf = planet.core_mass_fraction().unwrap_or(0.0);
    let rhr = planet.radioactive_heating_rate().unwrap_or(0.0);
    let age = universe_epoch + at_epoch;

    let core_r = conducting_core_radius(radius, planet.mass(), planet.kind(), cmf);
    let core_rho = core_density(planet.mass(), cmf, core_r);
    let q_cmb = core_mantle_boundary_heat_flux(planet.mass(), core_r, cmf, rhr, age);
    let q_conv = convective_core_heat_flux(planet.mass(), core_r, cmf, rhr, age);
    let q_rad = radiogenic_heat_flux(planet.mass(), rhr, age);

    let tidal_heat_flux = match (planet.orbital_parent(), planet.orbital_elements()) {
        (OrbitalParent::Fixed, _) | (_, None) => HeatFlux::new(0.0),
        (parent, Some(elements)) => {
            let parent_mass = if let Some(sys_id) = planet.star_system_id() {
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

            if parent_mass.value() > 0.0 {
                let mean_rho = planet_mean_density(&planet);
                let k2 = planet
                    .love_number_k2()
                    .unwrap_or_else(|| fallback_love_number_k2(planet.kind(), Some(mean_rho)));
                let q = planet
                    .tidal_dissipation_factor_q()
                    .unwrap_or_else(|| fallback_tidal_dissipation_factor_q(planet.kind()));

                tidal_heating_surface_flux(
                    parent_mass,
                    planet.mass(),
                    elements.semi_major_axis(),
                    elements.eccentricity(),
                    radius,
                    k2,
                    q,
                )
            } else {
                HeatFlux::new(0.0)
            }
        }
    };

    let r_c = core_r.value();
    let r_p = radius.value();
    let q_surf_core = if r_p > 0.0 {
        q_cmb.value() * (r_c * r_c) / (r_p * r_p)
    } else {
        q_cmb.value()
    };

    let mantle_mass_fraction = (1.0 - cmf).max(0.0);
    let q_surf_internal = HeatFlux::new(q_surf_core + q_rad.value() * mantle_mass_fraction * 4.0);

    let total_surf_q = total_surface_geothermal_heat_flux(q_surf_internal, tidal_heat_flux);

    Ok(PlanetaryCoreDiagnostic {
        core_radius: core_r,
        core_density: core_rho,
        cmb_heat_flux: q_cmb,
        convective_heat_flux: q_conv,
        radiogenic_heat_flux: q_rad,
        tidal_heat_flux,
        total_surface_heat_flux: total_surf_q,
    })
}
