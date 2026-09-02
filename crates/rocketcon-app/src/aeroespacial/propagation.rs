use crate::aeroespacial::aerodynamics::resolve_vehicle_aerodynamics;
use crate::aeroespacial::dynamics_step::evaluate_rigid_body_derivative;
use crate::aeroespacial::gravity::resolve_vehicle_gravitational_acceleration;
use crate::aeroespacial::vehicle::resolve_vehicle_snapshot;
use crate::environment::load_environment_snapshot;
use crate::error::{ RocketError, RocketResult };
use astronomicon_core::units::{ Angle, AngularMomentum, Duration, ForceVector, Vector3 };
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentDetails,
    ComponentOperationalState,
    ReactionWheelState,
    VehicleControlInput,
    VehiclePhysicalState,
};
use rocketcon_core::math::{
    gimbal_actuator_step,
    reaction_wheel_torque_and_momentum_delta,
    rk4_step,
    RigidBodyState,
};
use rocketcon_db::repositories::{
    operational_state as operational_state_repository,
    reaction_wheel_state as reaction_wheel_state_repository,
    vehicle as vehicle_repository,
    vehicle_physical_state as vehicle_physical_state_repository,
};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn advance_vehicle_physical_state(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput
) -> RocketResult<VehiclePhysicalState> {
    let physical_state = vehicle_physical_state_repository
        ::get_by_vehicle_id(pool, &vehicle_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let current_at_epoch = physical_state.captured_at_epoch();
    let new_at_epoch = current_at_epoch + dt;
    let reference_body_id = physical_state.reference_body_id();

    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;

    let snapshot = resolve_vehicle_snapshot(
        pool,
        vehicle_id,
        universe_epoch,
        current_at_epoch
    ).await?;

    let mut operational_states = HashMap::new();
    for (entry, _) in &components {
        if
            let Some(op) = operational_state_repository::get_by_vehicle_component_id(
                pool,
                &entry.id()
            ).await?
        {
            operational_states.insert(entry.id(), op);
        }
    }

    let mut reaction_wheel_states = HashMap::new();
    for (entry, _) in &components {
        if
            let Some(rw) = reaction_wheel_state_repository::get_by_vehicle_component_id(
                pool,
                &entry.id()
            ).await?
        {
            reaction_wheel_states.insert(entry.id(), rw);
        }
    }

    for (entry, record) in &components {
        if !snapshot.is_stage_active(entry.stage_index()) {
            continue;
        }

        if let ComponentDetails::Engine(engine) = record.details() {
            if
                let (Some(slew_rate), Some(cmd)) = (
                    engine.gimbal_slew_rate(),
                    control_input
                        .command_for(&entry.id())
                        .or_else(|| control_input.command_for(&entry.component_id())),
                )
            {
                if
                    let (Some(target_pitch), Some(target_yaw)) = (
                        cmd.target_gimbal_pitch,
                        cmd.target_gimbal_yaw,
                    )
                {
                    let current_op = operational_states.get(&entry.id()).copied();
                    let current_pitch = current_op
                        .and_then(|s| s.current_gimbal_pitch())
                        .unwrap_or(Angle::new(0.0));
                    let current_yaw = current_op
                        .and_then(|s| s.current_gimbal_yaw())
                        .unwrap_or(Angle::new(0.0));
                    let current_load = current_op.map(|s| s.load_fraction()).unwrap_or(1.0);

                    let (new_pitch, new_yaw) = gimbal_actuator_step(
                        current_pitch,
                        current_yaw,
                        target_pitch,
                        target_yaw,
                        slew_rate,
                        dt
                    );

                    let updated_op = ComponentOperationalState::new(
                        entry.id(),
                        current_load,
                        Some(new_pitch),
                        Some(new_yaw),
                        universe_epoch,
                        new_at_epoch
                    )?;
                    operational_state_repository::upsert(pool, &updated_op).await?;
                    operational_states.insert(entry.id(), updated_op);
                }
            }
        }
    }

    for (entry, record) in &components {
        if !snapshot.is_stage_active(entry.stage_index()) {
            continue;
        }

        if let ComponentDetails::ReactionWheel(rw) = record.details() {
            let axis = entry.actuation_axis().unwrap_or(Vector3::new(0.0, 0.0, 1.0)).normalized();
            let cmd = control_input
                .command_for(&entry.id())
                .or_else(|| control_input.command_for(&entry.component_id()));
            let torque_frac = cmd
                .and_then(|c| c.target_reaction_wheel_torque_fraction)
                .unwrap_or(0.0);
            let current_rw = reaction_wheel_states.get(&entry.id()).copied();
            let current_momentum = current_rw
                .map(|s| s.stored_angular_momentum())
                .unwrap_or(AngularMomentum::new(0.0));

            let (_, new_momentum) = reaction_wheel_torque_and_momentum_delta(
                rw,
                torque_frac,
                axis,
                current_momentum,
                dt
            );

            let updated_rw = ReactionWheelState::new(
                entry.id(),
                new_momentum,
                universe_epoch,
                new_at_epoch
            )?;
            reaction_wheel_state_repository::upsert(pool, &updated_rw).await?;
            reaction_wheel_states.insert(entry.id(), updated_rw);
        }
    }

    let environment = load_environment_snapshot(
        pool,
        reference_body_id,
        universe_epoch,
        current_at_epoch
    ).await?;

    let grav_acc = resolve_vehicle_gravitational_acceleration(
        pool,
        &environment,
        &physical_state,
        universe_epoch,
        current_at_epoch
    ).await?;

    let aero_diag = resolve_vehicle_aerodynamics(
        pool,
        &physical_state,
        reference_body_id,
        environment.planet_position.raw(),
        &components,
        snapshot.active_stages(),
        universe_epoch,
        current_at_epoch
    ).await?;

    let drag_force = aero_diag.map(|d| d.drag_force).unwrap_or(ForceVector::zero());

    let initial_rigid_state = RigidBodyState::from_physical_state(&physical_state);

    let new_rigid_state = rk4_step(&initial_rigid_state, dt, |sub_state| {
        evaluate_rigid_body_derivative(
            sub_state,
            snapshot.mass_properties(),
            &components,
            snapshot.active_stages(),
            &operational_states,
            &reaction_wheel_states,
            control_input,
            grav_acc,
            drag_force,
            dt
        )
    });

    let new_physical_state = VehiclePhysicalState::new(
        vehicle_id,
        new_rigid_state.position,
        new_rigid_state.velocity,
        new_rigid_state.orientation,
        new_rigid_state.angular_velocity,
        reference_body_id,
        universe_epoch,
        new_at_epoch
    )?;

    vehicle_physical_state_repository::upsert(pool, &new_physical_state).await?;

    Ok(new_physical_state)
}
