use crate::error::AppResult;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::error::{DomainError, DomainResult};
use astronomicon_core::math::gravity::{combined_gravitational_parameter, gravitational_parameter};
use astronomicon_core::math::kepler::orbital_position;
use astronomicon_core::units::{Duration, Mass, Position};
use astronomicon_db::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

pub fn compute_system_positions(
    stars: &[Star],
    planets: &[Planet],
    time_since_epoch: Duration,
) -> DomainResult<HashMap<Uuid, Position>> {
    let mut resolved_positions: HashMap<Uuid, Position> =
        HashMap::with_capacity(stars.len() + planets.len());

    let star_map: HashMap<Uuid, &Star> = stars.iter().map(|s| (s.id(), s)).collect();
    let planet_map: HashMap<Uuid, &Planet> = planets.iter().map(|p| (p.id(), p)).collect();

    let total_star_mass = Mass::new(stars.iter().map(|s| s.mass().value()).sum());

    for star in stars {
        if let Some(elements) = star.orbital_elements() {
            let mu = gravitational_parameter(total_star_mass);
            let rel_pos = orbital_position(&elements, mu, time_since_epoch)?;
            resolved_positions.insert(star.id(), rel_pos);
        } else {
            resolved_positions.insert(star.id(), Position::zero());
        }
    }

    fn resolve_planet(
        planet_id: Uuid,
        planet_map: &HashMap<Uuid, &Planet>,
        star_map: &HashMap<Uuid, &Star>,
        memo: &mut HashMap<Uuid, Position>,
        time_since_epoch: Duration,
    ) -> DomainResult<Position> {
        if let Some(&pos) = memo.get(&planet_id) {
            return Ok(pos);
        }

        let planet = planet_map.get(&planet_id).copied().ok_or_else(|| {
            DomainError::InvalidInvariant {
                field: "planet_id".to_string(),
                reason: format!("planet '{}' not found in system hierarchy", planet_id),
            }
        })?;

        let (parent_pos, parent_mass) = if let Some(parent_star_id) = planet.parent_star_id() {
            let star = star_map.get(&parent_star_id).copied().ok_or_else(|| {
                DomainError::InvalidInvariant {
                    field: "parent_star_id".to_string(),
                    reason: format!("parent star '{}' not found in system", parent_star_id),
                }
            })?;
            let pos = memo.get(&parent_star_id).copied().unwrap_or_else(Position::zero);
            (pos, star.mass())
        } else if let Some(parent_planet_id) = planet.parent_planet_id() {
            let parent_pos = resolve_planet(
                parent_planet_id,
                planet_map,
                star_map,
                memo,
                time_since_epoch,
            )?;
            let parent_planet =
                planet_map.get(&parent_planet_id).copied().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "parent_planet_id".to_string(),
                        reason: format!("parent planet '{}' not found in hierarchy", parent_planet_id),
                    }
                })?;
            (parent_pos, parent_planet.mass())
        } else {
            (Position::zero(), Mass::new(0.0))
        };

        let abs_pos = if let Some(elements) = planet.orbital_elements() {
            let mu = combined_gravitational_parameter(planet.mass(), parent_mass);
            let rel_pos = orbital_position(&elements, mu, time_since_epoch)?;
            parent_pos + rel_pos
        } else {
            parent_pos
        };

        memo.insert(planet_id, abs_pos);
        Ok(abs_pos)
    }

    for planet in planets {
        resolve_planet(
            planet.id(),
            &planet_map,
            &star_map,
            &mut resolved_positions,
            time_since_epoch,
        )?;
    }

    Ok(resolved_positions)
}

pub async fn resolve_system_positions(
    pool: &SqlitePool,
    star_system_id: Uuid,
    time_since_epoch: Duration,
) -> AppResult<HashMap<Uuid, Position>> {
    let star_rows =
        astronomicon_db::repositories::star_repository::list_by_system(pool, &star_system_id).await?;
    let stars = star_rows
        .into_iter()
        .map(Star::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let planet_rows =
        astronomicon_db::repositories::planet_repository::list_by_system(pool, &star_system_id).await?;
    let planets = planet_rows
        .into_iter()
        .map(Planet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let positions = compute_system_positions(&stars, &planets, time_since_epoch)?;
    Ok(positions)
}