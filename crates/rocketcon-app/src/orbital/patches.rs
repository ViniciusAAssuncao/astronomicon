use crate::error::{RocketError, RocketResult};
use crate::orbital::orbit::resolve_relative_state_for_body;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{planet_repository, star_repository};
use rocketcon_core::domain::TrajectoryPatch;
use rocketcon_db::repositories::{
    trajectory_patch as trajectory_patch_repository,
    vehicle_physical_state as vehicle_physical_state_repository,
};
use uuid::Uuid;

const COASTING_PATCH_LOOKAHEAD_S: f64 = 10.0;

fn patch_active_at(
    patch: &TrajectoryPatch,
    start_at_epoch: Option<Duration>,
    at_epoch: Duration,
    total_epoch: Duration,
) -> bool {
    if let Some(start) = start_at_epoch {
        let elapsed = at_epoch - start;
        return elapsed.value() >= 0.0
            && patch
                .end_universe_epoch()
                .is_none_or(|end| elapsed <= end - patch.start_universe_epoch());
    }
    patch.is_active_at(total_epoch)
}

pub async fn resolve_active_trajectory_patch(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<Option<TrajectoryPatch>> {
    Ok(
        resolve_active_trajectory_patch_with_local_epoch(
            pool,
            vehicle_id,
            universe_epoch,
            at_epoch,
        )
        .await?
        .map(|(patch, _)| patch),
    )
}

async fn resolve_active_trajectory_patch_with_local_epoch(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<Option<(TrajectoryPatch, Option<Duration>)>> {
    let total_epoch = universe_epoch + at_epoch;
    let patches =
        trajectory_patch_repository::list_for_vehicle_with_local_epoch(pool, &vehicle_id).await?;
    Ok(patches
        .into_iter()
        .find(|(patch, local_epoch)| patch_active_at(patch, *local_epoch, at_epoch, total_epoch)))
}

pub async fn invalidate_vehicle_trajectory_patches(
    pool: &SqlitePool,
    vehicle_id: Uuid,
) -> RocketResult<()> {
    trajectory_patch_repository::delete_patches_for_vehicle(pool, &vehicle_id).await?;
    Ok(())
}

pub async fn invalidate_future_trajectory_patches(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    from_epoch: Duration,
) -> RocketResult<()> {
    trajectory_patch_repository::delete_future_patches_after_epoch(pool, &vehicle_id, from_epoch)
        .await?;
    Ok(())
}

pub async fn generate_and_save_trajectory_patches(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<Vec<TrajectoryPatch>> {
    let state = vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or_else(|| {
            RocketError::Generic(format!(
                "physical state for vehicle '{}' not found",
                vehicle_id
            ))
        })?;
    let ref_id = state.reference_body_id();
    let system_id = if let Some(row) = planet_repository::get_by_id(pool, &ref_id).await? {
        let planet = Planet::try_from(row)?;
        let star =
            astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
        star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no system", star.id()))
        })?
    } else if let Some(row) = star_repository::get_by_id(pool, &ref_id).await? {
        let star = Star::try_from(row)?;
        star.star_system_id()
            .ok_or_else(|| RocketError::Generic(format!("star '{}' has no system", star.id())))?
    } else {
        return Err(RocketError::Generic(format!(
            "reference body '{}' not found",
            ref_id
        )));
    };
    let epoch = universe_epoch + at_epoch;
    let (position, velocity, mu) =
        resolve_relative_state_for_body(pool, &state, ref_id, system_id, epoch).await?;
    let elements =
        rocketcon_core::math::orbital::cartesian_to_osculating_elements(position, velocity, mu)?;
    let patch = TrajectoryPatch::from_osculating_elements(
        Uuid::new_v4(),
        vehicle_id,
        ref_id,
        epoch,
        Some(epoch + Duration::new(COASTING_PATCH_LOOKAHEAD_S)),
        &elements,
        mu,
    )?;
    trajectory_patch_repository::delete_patches_for_vehicle(pool, &vehicle_id).await?;
    trajectory_patch_repository::insert_patches(pool, std::slice::from_ref(&patch)).await?;
    trajectory_patch_repository::store_patch_local_epoch(pool, patch.id(), at_epoch).await?;
    Ok(vec![patch])
}
