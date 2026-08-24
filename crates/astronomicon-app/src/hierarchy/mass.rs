use crate::error::AppResult;
use crate::hierarchy::traversal::collect_stars_from_barycenter;
use astronomicon_core::domain::{MinorPlanet, OrbitalParent, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::units::Mass;
use astronomicon_db::repositories::{minor_planet_repository, planet_repository, star_repository};
use astronomicon_db::SqlitePool;
use std::collections::HashSet;

pub async fn resolve_parent_mass(pool: &SqlitePool, parent: &OrbitalParent) -> AppResult<Mass> {
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
        OrbitalParent::MinorPlanet(mp_id) => {
            let row = minor_planet_repository::get_by_id(pool, mp_id)
                .await?
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "parent_minor_planet_id".to_string(),
                    reason: format!("minor planet '{}' not found", mp_id),
                })?;
            let parent_mp = MinorPlanet::try_from(row)?;
            Ok(parent_mp.mass())
        }
        OrbitalParent::Barycenter(bary_id) => {
            let mut visited = HashSet::new();
            let stars = collect_stars_from_barycenter(pool, bary_id, &mut visited).await?;
            let total_mass: f64 = stars.iter().map(|s| s.mass().value()).sum();
            Ok(Mass::new(total_mass))
        }
    }
}
