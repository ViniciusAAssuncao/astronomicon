use crate::error::{RocketError, RocketResult};
use crate::orbital::orbit::resolve_relative_state_for_body;
use crate::orbital::patches::invalidate_vehicle_trajectory_patches;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::units::{
    Angle, Duration, GravitationalParameter, Length, Position, Speed, VelocityVector,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{planet_repository, star_repository};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::lambert_solver::{
    compute_porkchop_point, PorkchopPoint, TransferDirection,
};
use rocketcon_core::math::orbital::orbital_maneuvers::{
    apply_impulsive_delta_v, bi_elliptic_transfer, circularization_maneuver,
    hohmann_transfer, local_to_inertial_delta_v, node_plane_change_delta_v,
    orbital_insertion_delta_v, BiEllipticTransferResult, HohmannTransferResult,
    ManeuverDeltaV, ManeuverNode,
};
use rocketcon_db::repositories::vehicle_physical_state as vehicle_physical_state_repository;
use uuid::Uuid;

pub fn plan_hohmann_transfer(
    r_initial: Length,
    r_target: Length,
    mu: GravitationalParameter,
) -> RocketResult<HohmannTransferResult> {
    Ok(hohmann_transfer(r_initial, r_target, mu)?)
}

pub fn plan_bi_elliptic_transfer(
    r_initial: Length,
    r_target: Length,
    r_intermediate: Length,
    mu: GravitationalParameter,
) -> RocketResult<BiEllipticTransferResult> {
    Ok(bi_elliptic_transfer(r_initial, r_target, r_intermediate, mu)?)
}

pub fn plan_plane_change(
    velocity: VelocityVector,
    inclination_change: Angle,
) -> Speed {
    node_plane_change_delta_v(velocity, inclination_change)
}

pub fn plan_circularization(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
) -> ManeuverDeltaV {
    circularization_maneuver(position, velocity, mu)
}

pub fn plan_orbital_insertion(
    v_infinity: Speed,
    target_periapsis: Length,
    target_apoapsis: Option<Length>,
    mu: GravitationalParameter,
) -> RocketResult<Speed> {
    Ok(orbital_insertion_delta_v(v_infinity, target_periapsis, target_apoapsis, mu)?)
}

pub fn plan_interplanetary_lambert(
    departure_position: Position,
    departure_body_velocity: VelocityVector,
    arrival_position: Position,
    arrival_body_velocity: VelocityVector,
    time_of_flight: Duration,
    mu_central: GravitationalParameter,
    direction: TransferDirection,
) -> RocketResult<PorkchopPoint> {
    Ok(compute_porkchop_point(
        departure_position,
        departure_body_velocity,
        arrival_position,
        arrival_body_velocity,
        time_of_flight,
        mu_central,
        direction,
    )?)
}

pub async fn execute_impulsive_maneuver(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    delta_v_inertial: VelocityVector,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehiclePhysicalState> {
    let current_physical_state = vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let new_velocity = apply_impulsive_delta_v(current_physical_state.velocity(), delta_v_inertial);

    let new_state = VehiclePhysicalState::new(
        vehicle_id,
        current_physical_state.position(),
        new_velocity,
        current_physical_state.orientation(),
        current_physical_state.angular_velocity(),
        current_physical_state.reference_body_id(),
        universe_epoch,
        at_epoch,
    )?;

    vehicle_physical_state_repository::upsert(pool, &new_state).await?;
    invalidate_vehicle_trajectory_patches(pool, vehicle_id).await?;

    Ok(new_state)
}

pub async fn apply_maneuver_node(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    node: &ManeuverNode,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehiclePhysicalState> {
    let current_physical_state = vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let ref_id = current_physical_state.reference_body_id();

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

    let (rel_pos, rel_vel, _) = resolve_relative_state_for_body(pool, &current_physical_state, ref_id, system_id, total_epoch).await?;
    let delta_v_inertial = local_to_inertial_delta_v(node.delta_v, rel_pos, rel_vel);

    execute_impulsive_maneuver(pool, vehicle_id, delta_v_inertial, universe_epoch, at_epoch).await
}