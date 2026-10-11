use crate::error::{RocketError, RocketResult};
use crate::orbital::soi::resolve_body_state_at_epoch;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::units::{Duration, GravitationalParameter, Position, VelocityVector};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{planet_repository, star_repository};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::{
    cartesian_to_osculating_elements, OrbitType, OsculatingElements,
};
use rocketcon_db::repositories::vehicle_physical_state as vehicle_physical_state_repository;
use uuid::Uuid;

pub async fn resolve_relative_state_for_body(
    pool: &SqlitePool,
    physical_state: &VehiclePhysicalState,
    body_id: Uuid,
    system_id: Uuid,
    total_epoch: Duration,
) -> RocketResult<(Position, VelocityVector, GravitationalParameter)> {
    let (body_pos, body_vel) = resolve_body_state_at_epoch(pool, body_id, system_id, total_epoch).await?;

    let mu = if let Some(p_row) = planet_repository::get_by_id(pool, &body_id).await? {
        let planet = Planet::try_from(p_row)?;
        gravitational_parameter(planet.mass())
    } else if let Some(s_row) = star_repository::get_by_id(pool, &body_id).await? {
        let star = Star::try_from(s_row)?;
        gravitational_parameter(star.mass())
    } else {
        return Err(RocketError::Generic(format!("celestial body '{}' not found", body_id)));
    };

    let rel_pos = Position::from_raw(physical_state.position().raw() - body_pos.raw());
    let rel_vel = VelocityVector::from_raw(physical_state.velocity().raw() - body_vel.raw());

    Ok((rel_pos, rel_vel, mu))
}

pub async fn resolve_vehicle_osculating_elements(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<OsculatingElements> {
    let physical_state = vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let ref_id = physical_state.reference_body_id();

    let system_id = if let Some(p_row) = planet_repository::get_by_id(pool, &ref_id).await? {
        let planet = Planet::try_from(p_row)?;
        let star = astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
        star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no system", star.id()))
        })?
    } else if let Some(s_row) = star_repository::get_by_id(pool, &ref_id).await? {
        let star = Star::try_from(s_row)?;
        star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("star '{}' has no system", star.id()))
        })?
    } else {
        return Err(RocketError::Generic(format!("reference body '{}' not found", ref_id)));
    };

    let (rel_pos, rel_vel, mu) = resolve_relative_state_for_body(pool, &physical_state, ref_id, system_id, total_epoch).await?;
    let elements = cartesian_to_osculating_elements(rel_pos, rel_vel, mu)?;

    Ok(elements)
}

pub async fn resolve_vehicle_orbit_type(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<OrbitType> {
    let elements = resolve_vehicle_osculating_elements(pool, vehicle_id, universe_epoch, at_epoch).await?;
    Ok(elements.orbit_type())
}
