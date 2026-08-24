use crate::climate::{resolve_star_emission_profile, resolve_stellar_wind_at_planet};
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::geophysics::resolve_magnetic_field;
use crate::hierarchy::find_parent_star;
use astronomicon_core::domain::{Planet, StarKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::black_hole::{
    gravitational_redshift_between, gravitationally_redshifted_wavelength,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::radiation::{
    cutoff_rigidity, galactic_cosmic_ray_background, magnetosphere_shielding_factor,
    peak_wavelength, stellar_particle_flux,
};
use astronomicon_core::math::radiometry::orbital_irradiance;
use astronomicon_core::units::{
    Angle, Duration, Energy, Irradiance, MagneticRigidity, RadiationDose, Wavelength,
};
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InterplanetaryRadiationDiagnostic {
    pub peak_wavelength: Wavelength,
    pub stellar_irradiance: Irradiance,
    pub particle_flux: Irradiance,
    pub gcr_dose: RadiationDose,
    pub stellar_wind_dose: RadiationDose,
    pub lethal_uv_dose: RadiationDose,
    pub total_unshielded_dose: RadiationDose,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurfaceRadiationDiagnostic {
    pub equatorial_cutoff_rigidity: MagneticRigidity,
    pub polar_cutoff_rigidity: MagneticRigidity,
    pub equatorial_magnetic_shielding: f64,
    pub polar_magnetic_shielding: f64,
    pub atmospheric_transmission: f64,
    pub equatorial_surface_dose: RadiationDose,
    pub polar_surface_dose: RadiationDose,
}

pub async fn resolve_interplanetary_radiation(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<InterplanetaryRadiationDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, planet.orbital_parent()).await?;

    let (star_lum, star_temp, r_emit) =
        resolve_star_emission_profile(pool, &star, universe_epoch, at_epoch).await?;

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

    let (peak_lambda, z_factor) = if star.kind() == StarKind::BlackHole {
        let base_lambda = peak_wavelength(star_temp);
        let red_lambda = gravitationally_redshifted_wavelength(
            base_lambda,
            star.mass(),
            r_emit,
            orbital_distance,
        );
        let z = gravitational_redshift_between(star.mass(), r_emit, orbital_distance);
        (red_lambda, z)
    } else {
        (peak_wavelength(star_temp), 1.0)
    };

    let base_irradiance = orbital_irradiance(star_lum, orbital_distance);
    let top_irradiance = Irradiance::new(base_irradiance.value() / (z_factor * z_factor));

    let stellar_wind = resolve_stellar_wind_at_planet(
        pool,
        planet_id,
        1.0,
        1.0,
        universe_epoch,
        at_epoch,
    )
    .await?;

    let part_flux = stellar_particle_flux(
        stellar_wind.wind_density_at_orbit,
        stellar_wind.terminal_wind_speed,
    );

    let gcr = galactic_cosmic_ray_background();
    let sw_dose = RadiationDose::new(part_flux.value() * 1000.0);

    let uv_fraction = if peak_lambda.value() > 0.0 {
        (280e-9 / peak_lambda.value()).powi(4).clamp(0.001, 0.5)
    } else {
        0.01
    };
    let uv_dose = RadiationDose::new(top_irradiance.value() * uv_fraction * 0.005);
    let total_dose = gcr + sw_dose + uv_dose;

    Ok(InterplanetaryRadiationDiagnostic {
        peak_wavelength: peak_lambda,
        stellar_irradiance: top_irradiance,
        particle_flux: part_flux,
        gcr_dose: gcr,
        stellar_wind_dose: sw_dose,
        lethal_uv_dose: uv_dose,
        total_unshielded_dose: total_dose,
    })
}

pub async fn resolve_surface_radiation(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<SurfaceRadiationDiagnostic> {
    let interplanetary = resolve_interplanetary_radiation(
        pool,
        planet_id,
        universe_epoch,
        at_epoch,
    )
    .await?;

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

    let magnetosphere = resolve_magnetic_field(
        pool,
        planet_id,
        1.0,
        1.0,
        universe_epoch,
        at_epoch,
    )
    .await?;

    let rc_eq = cutoff_rigidity(magnetosphere.dipole_moment, radius, Angle::new(0.0));
    let rc_pol = cutoff_rigidity(magnetosphere.dipole_moment, radius, Angle::new(PI / 2.0));

    let particle_energy = Energy::new(1.0e9);
    let t_mag_eq = magnetosphere_shielding_factor(rc_eq, particle_energy);
    let t_mag_pol = magnetosphere_shielding_factor(rc_pol, particle_energy);

    let atm_trans = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atm) => {
            let mu_planet = gravitational_parameter(planet.mass());
            let g = surface_gravity(mu_planet, radius);
            atm.radiation_transmission(g)?
        }
        None => 1.0,
    };

    let particulate_dose = interplanetary.gcr_dose + interplanetary.stellar_wind_dose;
    let incident_eq = particulate_dose.value() * t_mag_eq + interplanetary.lethal_uv_dose.value();
    let incident_pol = particulate_dose.value() * t_mag_pol + interplanetary.lethal_uv_dose.value();

    let eq_surface_dose = RadiationDose::new(incident_eq * atm_trans);
    let pol_surface_dose = RadiationDose::new(incident_pol * atm_trans);

    Ok(SurfaceRadiationDiagnostic {
        equatorial_cutoff_rigidity: rc_eq,
        polar_cutoff_rigidity: rc_pol,
        equatorial_magnetic_shielding: t_mag_eq,
        polar_magnetic_shielding: t_mag_pol,
        atmospheric_transmission: atm_trans,
        equatorial_surface_dose: eq_surface_dose,
        polar_surface_dose: pol_surface_dose,
    })
}
