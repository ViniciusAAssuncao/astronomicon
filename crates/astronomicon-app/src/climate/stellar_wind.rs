use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use astronomicon_core::domain::{Planet, StarKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::math::radiometry::{escape_velocity, stellar_luminosity};
use astronomicon_core::math::stellar_wind::{
    reimers_mass_loss_rate, stellar_wind_density, stellar_wind_dynamic_pressure,
    terminal_wind_speed,
};
use astronomicon_core::units::{
    Density, Duration, Length, Mass, MassRate, Pressure, Speed, Temperature,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StellarWindDiagnostic {
    pub mass_loss_rate: MassRate,
    pub escape_velocity: Speed,
    pub terminal_wind_speed: Speed,
    pub wind_density_at_orbit: Density,
    pub dynamic_pressure: Pressure,
}

pub fn resolve_stellar_wind_at_distance(
    star_mass: Mass,
    star_radius: Length,
    star_temp: Temperature,
    orbital_distance: Length,
    eta: f64,
    wind_scaling: f64,
) -> StellarWindDiagnostic {
    let star_lum = stellar_luminosity(star_radius, star_temp);
    let mu_star = gravitational_parameter(star_mass);
    let v_esc = escape_velocity(mu_star, star_radius);

    let m_dot = reimers_mass_loss_rate(star_lum, star_radius, star_mass, eta);
    let v_inf = terminal_wind_speed(v_esc, wind_scaling);
    let rho_sw = stellar_wind_density(m_dot, v_inf, orbital_distance);
    let p_dyn = stellar_wind_dynamic_pressure(rho_sw, v_inf);

    StellarWindDiagnostic {
        mass_loss_rate: m_dot,
        escape_velocity: v_esc,
        terminal_wind_speed: v_inf,
        wind_density_at_orbit: rho_sw,
        dynamic_pressure: p_dyn,
    }
}

pub async fn resolve_stellar_wind_at_planet(
    pool: &SqlitePool,
    planet_id: Uuid,
    eta: f64,
    wind_scaling: f64,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<StellarWindDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, planet.orbital_parent()).await?;

    if star.kind() == StarKind::BlackHole {
        return Ok(StellarWindDiagnostic {
            mass_loss_rate: MassRate::new(0.0),
            escape_velocity: Speed::new(0.0),
            terminal_wind_speed: Speed::new(0.0),
            wind_density_at_orbit: Density::new(0.0),
            dynamic_pressure: Pressure::new(0.0),
        });
    }

    let star_temp = star
        .effective_temperature()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "effective_temperature".to_string(),
            reason: "star does not have effective temperature".to_string(),
        })?;

    let star_radius = star.radius().ok_or_else(|| DomainError::InvalidInvariant {
        field: "radius".to_string(),
        reason: "star does not have radius".to_string(),
    })?;

    let system_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let positions = resolve_system_positions(pool, system_id, total_epoch).await?;

    let planet_pos =
        positions
            .get(&planet.id())
            .copied()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "planet_id".to_string(),
                reason: format!(
                    "position for planet '{}' could not be resolved",
                    planet.id()
                ),
            })?;

    let star_pos =
        positions
            .get(&star.id())
            .copied()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "star_id".to_string(),
                reason: format!("position for star '{}' could not be resolved", star.id()),
            })?;

    let orbital_distance = (planet_pos - star_pos).magnitude();

    Ok(resolve_stellar_wind_at_distance(
        star.mass(),
        star_radius,
        star_temp,
        orbital_distance,
        eta,
        wind_scaling,
    ))
}
