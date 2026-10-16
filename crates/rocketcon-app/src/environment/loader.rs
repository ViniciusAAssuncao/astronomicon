use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository, star_repository};
use rocketcon_core::environment::EnvironmentSnapshot;
use uuid::Uuid;

pub async fn load_environment_snapshot(
    pool: &SqlitePool,
    body_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<EnvironmentSnapshot> {
    let (environment, _) =
        load_environment_snapshot_with_catalog(pool, body_id, universe_epoch, at_epoch).await?;
    Ok(environment)
}

pub(crate) async fn load_environment_snapshot_with_catalog(
    pool: &SqlitePool,
    body_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<(EnvironmentSnapshot, EphemerisCatalog)> {
    let total_epoch = universe_epoch + at_epoch;
    if let Some(row) = planet_repository::get_by_id(pool, &body_id).await? {
        let planet = Planet::try_from(row)?;
        let star =
            astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
        let system_id = star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no star system", star.id()))
        })?;
        let catalog = EphemerisCatalog::load(pool, system_id).await?;
        let positions = catalog
            .positions(total_epoch)
            .map_err(astronomicon_app::error::AppError::from)?;
        let planet_position = positions.get(&planet.id()).copied().ok_or_else(|| {
            RocketError::Generic(format!("position for planet '{}' missing", planet.id()))
        })?;
        let star_position = positions.get(&star.id()).copied().ok_or_else(|| {
            RocketError::Generic(format!("position for star '{}' missing", star.id()))
        })?;
        let atmosphere = atmosphere_repository::get_by_planet_id(pool, &body_id).await?;
        let environment = EnvironmentSnapshot::new(
            star,
            Some(planet),
            atmosphere,
            star_position,
            Some(planet_position),
            system_id,
            universe_epoch,
            at_epoch,
        )?;
        return Ok((environment, catalog));
    }
    if let Some(row) = star_repository::get_by_id(pool, &body_id).await? {
        let star = Star::try_from(row)?;
        let system_id = star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("star '{}' has no star system", star.id()))
        })?;
        let catalog = EphemerisCatalog::load(pool, system_id).await?;
        let positions = catalog
            .positions(total_epoch)
            .map_err(astronomicon_app::error::AppError::from)?;
        let star_position = positions.get(&star.id()).copied().ok_or_else(|| {
            RocketError::Generic(format!("position for star '{}' missing", star.id()))
        })?;
        let environment = EnvironmentSnapshot::new(
            star,
            None,
            None,
            star_position,
            None,
            system_id,
            universe_epoch,
            at_epoch,
        )?;
        return Ok((environment, catalog));
    }
    Err(RocketError::Generic(format!(
        "reference body '{}' not found",
        body_id
    )))
}
