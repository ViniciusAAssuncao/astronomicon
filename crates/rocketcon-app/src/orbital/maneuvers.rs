use crate::error::{ RocketError, RocketResult };
use crate::orbital::orbit::resolve_relative_state_for_body;
use crate::orbital::patches::invalidate_vehicle_trajectory_patches;
use crate::orbital::soi::resolve_body_state_at_epoch;
use astronomicon_core::domain::{ Planet, Star };
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::units::{
    Angle,
    Duration,
    GravitationalParameter,
    Length,
    Position,
    Speed,
    Vector3,
    VelocityVector,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{ planet_repository, star_repository };
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::gravity_assist::{
    solve_gravity_assist_flyby,
    FlybyWaypointPlan,
    GravityAssistTourPlan,
};
use rocketcon_core::math::orbital::lambert_solver::{
    compute_porkchop_point,
    solve_lambert,
    PorkchopPoint,
    TransferDirection,
};
use rocketcon_core::math::orbital::orbital_maneuvers::{
    apply_impulsive_delta_v,
    bi_elliptic_transfer,
    circularization_maneuver,
    hohmann_transfer,
    local_to_inertial_delta_v,
    node_plane_change_delta_v,
    orbital_insertion_delta_v,
    BiEllipticTransferResult,
    HohmannTransferResult,
    ManeuverDeltaV,
    ManeuverNode,
};
use rocketcon_db::repositories::vehicle_physical_state as vehicle_physical_state_repository;
use uuid::Uuid;

pub fn plan_hohmann_transfer(
    r_initial: Length,
    r_target: Length,
    mu: GravitationalParameter
) -> RocketResult<HohmannTransferResult> {
    Ok(hohmann_transfer(r_initial, r_target, mu)?)
}

pub fn plan_bi_elliptic_transfer(
    r_initial: Length,
    r_target: Length,
    r_intermediate: Length,
    mu: GravitationalParameter
) -> RocketResult<BiEllipticTransferResult> {
    Ok(bi_elliptic_transfer(r_initial, r_target, r_intermediate, mu)?)
}

pub fn plan_plane_change(velocity: VelocityVector, inclination_change: Angle) -> Speed {
    node_plane_change_delta_v(velocity, inclination_change)
}

pub fn plan_circularization(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter
) -> ManeuverDeltaV {
    circularization_maneuver(position, velocity, mu)
}

pub fn plan_orbital_insertion(
    v_infinity: Speed,
    target_periapsis: Length,
    target_apoapsis: Option<Length>,
    mu: GravitationalParameter
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
    direction: TransferDirection
) -> RocketResult<PorkchopPoint> {
    Ok(
        compute_porkchop_point(
            departure_position,
            departure_body_velocity,
            arrival_position,
            arrival_body_velocity,
            time_of_flight,
            mu_central,
            direction
        )?
    )
}

pub async fn plan_gravity_assist_tour(
    pool: &SqlitePool,
    departure_body_id: Uuid,
    intermediate_body_ids: &[Uuid],
    destination_body_id: Uuid,
    departure_epoch: Duration,
    leg_durations: &[Duration]
) -> RocketResult<GravityAssistTourPlan> {
    let n_legs = intermediate_body_ids.len() + 1;
    if leg_durations.len() != n_legs {
        return Err(
            RocketError::Generic(
                format!("expected {} leg durations, found {}", n_legs, leg_durations.len())
            )
        );
    }

    let mut body_ids = Vec::with_capacity(n_legs + 1);
    body_ids.push(departure_body_id);
    body_ids.extend_from_slice(intermediate_body_ids);
    body_ids.push(destination_body_id);

    let mut epochs = Vec::with_capacity(n_legs + 1);
    let mut cur_epoch = departure_epoch;
    epochs.push(cur_epoch);
    for &dur in leg_durations {
        cur_epoch = cur_epoch + dur;
        epochs.push(cur_epoch);
    }

    let ref_id = departure_body_id;
    let (system_id, star) = if let Some(p_row) = planet_repository::get_by_id(pool, &ref_id).await? {
        let planet = Planet::try_from(p_row)?;
        let star = astronomicon_app::hierarchy::find_parent_star(
            pool,
            planet.orbital_parent()
        ).await?;
        let s_id = star
            .star_system_id()
            .ok_or_else(|| {
                RocketError::Generic(format!("parent star '{}' has no system", star.id()))
            })?;
        (s_id, star)
    } else if let Some(s_row) = star_repository::get_by_id(pool, &ref_id).await? {
        let star = Star::try_from(s_row)?;
        let s_id = star
            .star_system_id()
            .ok_or_else(|| {
                RocketError::Generic(format!("star '{}' has no system", star.id()))
            })?;
        (s_id, star)
    } else {
        return Err(RocketError::Generic(format!("body '{}' not found", ref_id)));
    };

    let mu_central = gravitational_parameter(star.mass());

    let mut body_states = Vec::with_capacity(body_ids.len());
    for (&id, &epoch) in body_ids.iter().zip(epochs.iter()) {
        let (pos, vel) = resolve_body_state_at_epoch(pool, id, system_id, epoch).await?;
        body_states.push((pos, vel));
    }

    let mut leg_solutions = Vec::with_capacity(n_legs);
    for k in 0..n_legs {
        let (r1, _) = body_states[k];
        let (r2, _) = body_states[k + 1];
        let dt = leg_durations[k];
        let sol = solve_lambert(r1, r2, dt, mu_central, TransferDirection::ShortWay)?;
        leg_solutions.push(sol);
    }

    let dep_rel_v = leg_solutions[0].departure_velocity.raw() - body_states[0].1.raw();
    let departure_delta_v = Speed::new(dep_rel_v.magnitude());

    let arr_rel_v = leg_solutions[n_legs - 1].arrival_velocity.raw() - body_states[n_legs].1.raw();
    let arrival_delta_v = Speed::new(arr_rel_v.magnitude());

    let ref_pole = Vector3::new(0.0, 0.0, 1.0);
    let mut waypoints = Vec::with_capacity(intermediate_body_ids.len());
    let mut flyby_dv_sum = 0.0;

    for (i, &int_id) in intermediate_body_ids.iter().enumerate() {
        let p_row = planet_repository
            ::get_by_id(pool, &int_id).await?
            .ok_or_else(|| {
                RocketError::Generic(format!("flyby planet '{}' not found", int_id))
            })?;
        let planet = Planet::try_from(p_row)?;
        let planet_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
        let mu_planet = gravitational_parameter(planet.mass());
        let min_altitude = Length::new(100_000.0);

        let v_in = leg_solutions[i].arrival_velocity;
        let v_out = leg_solutions[i + 1].departure_velocity;
        let v_planet = body_states[i + 1].1;

        let flyby = solve_gravity_assist_flyby(
            int_id,
            v_in,
            v_out,
            v_planet,
            planet_radius,
            min_altitude,
            mu_planet,
            ref_pole
        )?;

        flyby_dv_sum += flyby.deflection.delta_v_periapsis.value();
        waypoints.push(FlybyWaypointPlan {
            body_id: int_id,
            encounter_epoch: epochs[i + 1],
            flyby,
        });
    }

    let total_dur = cur_epoch - departure_epoch;
    let total_dv = departure_delta_v.value() + flyby_dv_sum + arrival_delta_v.value();

    Ok(GravityAssistTourPlan {
        departure_body_id,
        destination_body_id,
        departure_epoch,
        total_duration: total_dur,
        departure_delta_v,
        arrival_delta_v,
        total_mission_delta_v: Speed::new(total_dv),
        waypoints,
    })
}

pub async fn execute_impulsive_maneuver(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    delta_v_inertial: VelocityVector,
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<VehiclePhysicalState> {
    let current_physical_state = vehicle_physical_state_repository
        ::get_by_vehicle_id(pool, &vehicle_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let new_velocity = apply_impulsive_delta_v(current_physical_state.velocity(), delta_v_inertial);

    let new_state = VehiclePhysicalState::new_with_max_q(
        vehicle_id,
        current_physical_state.position(),
        new_velocity,
        current_physical_state.orientation(),
        current_physical_state.angular_velocity(),
        current_physical_state.reference_body_id(),
        universe_epoch,
        at_epoch,
        current_physical_state.max_dynamic_pressure(),
        current_physical_state.max_dynamic_pressure_epoch()
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
    at_epoch: Duration
) -> RocketResult<VehiclePhysicalState> {
    let current_physical_state = vehicle_physical_state_repository
        ::get_by_vehicle_id(pool, &vehicle_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let ref_id = current_physical_state.reference_body_id();

    let system_id = if let Some(p_row) = planet_repository::get_by_id(pool, &ref_id).await? {
        let planet = Planet::try_from(p_row)?;
        let star = astronomicon_app::hierarchy::find_parent_star(
            pool,
            planet.orbital_parent()
        ).await?;
        star
            .star_system_id()
            .ok_or_else(|| {
                RocketError::Generic(format!("parent star '{}' has no system", star.id()))
            })?
    } else if let Some(s_row) = star_repository::get_by_id(pool, &ref_id).await? {
        let star = Star::try_from(s_row)?;
        star
            .star_system_id()
            .ok_or_else(|| { RocketError::Generic(format!("star '{}' has no system", star.id())) })?
    } else {
        return Err(RocketError::Generic(format!("reference body '{}' not found", ref_id)));
    };

    let (rel_pos, rel_vel, _) = resolve_relative_state_for_body(
        pool,
        &current_physical_state,
        ref_id,
        system_id,
        total_epoch
    ).await?;
    let delta_v_inertial = local_to_inertial_delta_v(node.delta_v, rel_pos, rel_vel);

    execute_impulsive_maneuver(pool, vehicle_id, delta_v_inertial, universe_epoch, at_epoch).await
}
