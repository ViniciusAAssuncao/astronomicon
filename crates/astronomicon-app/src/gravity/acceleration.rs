use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::gravity::hierarchy_view::SystemHierarchy;
use astronomicon_core::math::gravity::gravitational_acceleration_at;
use astronomicon_core::units::{AccelerationVector, Duration, Position};
use astronomicon_db::SqlitePool;
use uuid::Uuid;

pub async fn resolve_net_gravitational_acceleration(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    point: Position,
    at_epoch: Duration,
) -> AppResult<AccelerationVector> {
    let hierarchy = SystemHierarchy::load(pool, star_system_id).await?;
    let positions = resolve_system_positions(pool, *star_system_id, at_epoch).await?;
    let mut sources = Vec::with_capacity(
        hierarchy.stars.len() + hierarchy.planets.len() + hierarchy.minor_planets.len(),
    );

    for star in &hierarchy.stars {
        if let Some(&pos) = positions.get(&star.id()) {
            sources.push((pos, star.mass()));
        }
    }

    for planet in &hierarchy.planets {
        if let Some(&pos) = positions.get(&planet.id()) {
            sources.push((pos, planet.mass()));
        }
    }

    for mp in &hierarchy.minor_planets {
        if let Some(&pos) = positions.get(&mp.id()) {
            sources.push((pos, mp.mass()));
        }
    }

    Ok(gravitational_acceleration_at(point, &sources))
}
