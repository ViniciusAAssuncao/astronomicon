use crate::error::AppResult;
use crate::gravity::hierarchy_view::SystemHierarchy;
use crate::shape::{planet_mean_density, star_mean_density};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::math::minor_planet::{
    bulk_density, equivalent_spherical_radius, grain_density_by_spectral_type,
};
use astronomicon_core::math::tidal::{
    roche_limit_fluid, roche_limit_rigid, synchronous_orbit_radius,
};
use astronomicon_core::units::Length;
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RocheLimits {
    pub rigid: Length,
    pub fluid: Length,
}

pub async fn resolve_roche_limits(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    primary_id: &Uuid,
    satellite_id: &Uuid,
) -> AppResult<RocheLimits> {
    let hierarchy = SystemHierarchy::load(pool, star_system_id).await?;

    let (primary_density, primary_radius) =
        if let Some(star) = hierarchy.stars.iter().find(|s| s.id() == *primary_id) {
            let r = star.radius().ok_or_else(|| DomainError::InvalidInvariant {
                field: "radius".to_string(),
                reason: format!("primary star '{}' has no radius", primary_id),
            })?;
            (star_mean_density(star), r)
        } else if let Some(planet) = hierarchy.planets.iter().find(|p| p.id() == *primary_id) {
            let r = planet
                .equatorial_radius()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "equatorial_radius".to_string(),
                    reason: format!("primary planet '{}' has no equatorial radius", primary_id),
                })?;
            (planet_mean_density(planet), r)
        } else if let Some(mp) = hierarchy.minor_planets.iter().find(|m| m.id() == *primary_id) {
            let grain_rho = grain_density_by_spectral_type(mp.spectral_type());
            let bulk_rho = bulk_density(grain_rho, mp.macroporosity().unwrap_or(0.0));
            let r = match (mp.axis_a(), mp.axis_b(), mp.axis_c()) {
                (Some(a), Some(b), Some(c)) => equivalent_spherical_radius(a, b, c),
                _ => {
                    let vol = mp.mass().value() / bulk_rho.value().max(1.0);
                    Length::new((3.0 * vol / (4.0 * PI)).cbrt())
                }
            };
            (bulk_rho, r)
        } else {
            return Err(DomainError::InvalidInvariant {
                field: "primary_id".to_string(),
                reason: format!(
                    "primary entity '{}' not found in system '{}'",
                    primary_id, star_system_id
                ),
            }
            .into());
        };

    let satellite_density = if let Some(star) =
        hierarchy.stars.iter().find(|s| s.id() == *satellite_id)
    {
        star_mean_density(star)
    } else if let Some(planet) = hierarchy.planets.iter().find(|p| p.id() == *satellite_id) {
        planet_mean_density(planet)
    } else if let Some(mp) = hierarchy.minor_planets.iter().find(|m| m.id() == *satellite_id) {
        let grain_rho = grain_density_by_spectral_type(mp.spectral_type());
        bulk_density(grain_rho, mp.macroporosity().unwrap_or(0.0))
    } else {
        return Err(DomainError::InvalidInvariant {
            field: "satellite_id".to_string(),
            reason: format!(
                "satellite entity '{}' not found in system '{}'",
                satellite_id, star_system_id
            ),
        }
        .into());
    };

    let rigid = roche_limit_rigid(primary_radius, primary_density, satellite_density);
    let fluid = roche_limit_fluid(primary_radius, primary_density, satellite_density);

    Ok(RocheLimits { rigid, fluid })
}

pub async fn resolve_synchronous_orbit_radius(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    primary_id: &Uuid,
) -> AppResult<Length> {
    let hierarchy = SystemHierarchy::load(pool, star_system_id).await?;

    let (mass, rotation_period) =
        if let Some(star) = hierarchy.stars.iter().find(|s| s.id() == *primary_id) {
            let rot = star
                .rotation_period()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "rotation_period".to_string(),
                    reason: format!("star '{}' has no rotation period", primary_id),
                })?;
            (star.mass(), rot)
        } else if let Some(planet) = hierarchy.planets.iter().find(|p| p.id() == *primary_id) {
            let rot = planet
                .rotation_period()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "rotation_period".to_string(),
                    reason: format!("planet '{}' has no rotation period", primary_id),
                })?;
            (planet.mass(), rot)
        } else if let Some(mp) = hierarchy.minor_planets.iter().find(|m| m.id() == *primary_id) {
            let rot = mp
                .rotation_period()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "rotation_period".to_string(),
                    reason: format!("minor planet '{}' has no rotation period", primary_id),
                })?;
            (mp.mass(), rot)
        } else {
            return Err(DomainError::InvalidInvariant {
                field: "primary_id".to_string(),
                reason: format!(
                    "entity '{}' not found in system '{}'",
                    primary_id, star_system_id
                ),
            }
            .into());
        };

    let mu = gravitational_parameter(mass);
    Ok(synchronous_orbit_radius(mu, rotation_period))
}
