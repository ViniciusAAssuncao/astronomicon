use crate::climate::resolve_stellar_wind_at_planet;
use crate::error::AppResult;
use crate::gravity::resolve_entity_effective_mass;
use crate::shape::planet_mean_density;
use astronomicon_core::domain::{OrbitalParent, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::geophysics::{
    conducting_core_radius, convective_core_heat_flux, core_density,
    core_mantle_boundary_heat_flux, radiogenic_heat_flux, total_surface_geothermal_heat_flux,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::magnetic_field::{
    convective_magnetic_dipole_moment, equatorial_surface_magnetic_field, magnetopause_radius,
    polar_surface_magnetic_field, specific_buoyancy_flux,
};
use astronomicon_core::math::rotation::angular_velocity_from_rotation_period;
use astronomicon_core::math::tidal::{
    fallback_love_number_k2, fallback_tidal_dissipation_factor_q, tidal_heating_surface_flux,
};
use astronomicon_core::units::constants::VACUUM_PERMEABILITY;
use astronomicon_core::units::{
    Density, Duration, HeatFlux, Length, MagneticDipoleMoment, MagneticFluxDensity, Mass,
};
use astronomicon_db::repositories::{planet_repository, star_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MagnetosphereDiagnostic {
    pub dipole_moment: MagneticDipoleMoment,
    pub equatorial_magnetic_field: MagneticFluxDensity,
    pub polar_magnetic_field: MagneticFluxDensity,
    pub magnetopause_radius: Length,
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
    let q_surf_internal = if r_p > 0.0 {
        HeatFlux::new(q_cmb.value() * (r_c * r_c) / (r_p * r_p))
    } else {
        q_cmb
    };

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

pub async fn resolve_magnetic_field(
    pool: &SqlitePool,
    planet_id: Uuid,
    eta: f64,
    wind_scaling: f64,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<MagnetosphereDiagnostic> {
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

    let dipole_moment = if let Some(b_locked) = planet.magnetic_field_locked() {
        let r3 = radius.value().powi(3);
        let m_val = (4.0 * PI * r3 * b_locked.value()) / VACUUM_PERMEABILITY;
        MagneticDipoleMoment::new(m_val)
    } else {
        let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await?;
        let cmf = planet.core_mass_fraction().unwrap_or(0.0);
        let m_c = Mass::new(planet.mass().value() * cmf);
        let mu_c = gravitational_parameter(m_c);
        let g_c = surface_gravity(mu_c, core_diag.core_radius);

        let alpha = 1.0e-5;
        let cp = 800.0;
        let buoyancy_flux = specific_buoyancy_flux(
            core_diag.convective_heat_flux,
            g_c,
            core_diag.core_density,
            alpha,
            cp,
        );

        let rot_period = planet
            .rotation_period()
            .unwrap_or_else(|| Duration::new(86400.0));
        let omega = angular_velocity_from_rotation_period(rot_period);

        convective_magnetic_dipole_moment(
            core_diag.core_radius,
            core_diag.core_density,
            buoyancy_flux,
            omega,
        )
    };

    let b_eq = equatorial_surface_magnetic_field(dipole_moment, radius);
    let b_pol = polar_surface_magnetic_field(dipole_moment, radius);

    let stellar_wind = resolve_stellar_wind_at_planet(
        pool,
        planet_id,
        eta,
        wind_scaling,
        universe_epoch,
        at_epoch,
    )
    .await?;

    let r_mp = magnetopause_radius(dipole_moment, stellar_wind.dynamic_pressure);

    Ok(MagnetosphereDiagnostic {
        dipole_moment,
        equatorial_magnetic_field: b_eq,
        polar_magnetic_field: b_pol,
        magnetopause_radius: r_mp,
    })
}
