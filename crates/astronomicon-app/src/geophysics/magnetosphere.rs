use crate::climate::resolve_stellar_wind_at_planet;
use crate::error::AppResult;
use crate::geophysics::core::resolve_planetary_core;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::magnetic_field::{
    convective_magnetic_dipole_moment, equatorial_surface_magnetic_field, magnetopause_radius,
    polar_surface_magnetic_field, specific_buoyancy_flux,
};
use astronomicon_core::math::rotation::angular_velocity_from_rotation_period;
use astronomicon_core::units::constants::VACUUM_PERMEABILITY;
use astronomicon_core::units::{Duration, Length, MagneticDipoleMoment, MagneticFluxDensity, Mass};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MagnetosphereDiagnostic {
    pub dipole_moment: MagneticDipoleMoment,
    pub equatorial_magnetic_field: MagneticFluxDensity,
    pub polar_magnetic_field: MagneticFluxDensity,
    pub magnetopause_radius: Length,
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
