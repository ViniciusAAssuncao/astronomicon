use crate::error::{RocketError, RocketResult};
use astronomicon_core::domain::Planet;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository};
use rocketcon_core::environment::EnvironmentSnapshot;
use uuid::Uuid;

pub async fn load_environment_snapshot(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<EnvironmentSnapshot> {
    let row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| RocketError::Generic(format!("planet '{}' not found", planet_id)))?;
    let planet = Planet::try_from(row)?;

    let star = astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;

    let system_id = star.star_system_id().ok_or_else(|| {
        RocketError::Generic(format!(
            "parent star '{}' has no associated star system",
            star.id()
        ))
    })?;

    let total_epoch = universe_epoch + at_epoch;
    let positions =
        astronomicon_app::ephemeris::resolve_system_positions(pool, system_id, total_epoch).await?;

    let planet_position = positions
        .get(&planet.id())
        .copied()
        .ok_or_else(|| {
            RocketError::Generic(format!(
                "position for planet '{}' could not be resolved",
                planet.id()
            ))
        })?;

    let star_position = positions
        .get(&star.id())
        .copied()
        .ok_or_else(|| {
            RocketError::Generic(format!(
                "position for star '{}' could not be resolved",
                star.id()
            ))
        })?;

    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let snapshot = EnvironmentSnapshot::new(
        star,
        planet,
        atmosphere,
        star_position,
        planet_position,
        system_id,
        universe_epoch,
        at_epoch,
    )?;

    Ok(snapshot)
}
