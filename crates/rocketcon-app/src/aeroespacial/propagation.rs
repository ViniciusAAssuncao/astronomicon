use crate::aeroespacial::aerodynamics::{
    AerodynamicDiagnostic, resolve_vehicle_aerodynamics,
    resolve_vehicle_aerodynamics_in_existing_transaction,
};
use crate::aeroespacial::dynamics_step::evaluate_rigid_body_derivative_with_ambient_pressure;
use crate::aeroespacial::gravity::GravityField;
use crate::aeroespacial::max_dynamic_pressure::resolve_max_q;
use crate::aeroespacial::vehicle::resolve_vehicle_snapshot;
use crate::environment::load_environment_snapshot;
use crate::error::{RocketError, RocketResult};
use crate::orbital::nbody_field::CatalogGravityField;
use crate::orbital::nbody_impact::CatalogImpactMonitor;
use crate::orbital::nbody_powered::integrate_nbody_rigid_body;
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Angle, AngularMomentum, Duration, ForceVector, Pressure, Vector3};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentDetails, ComponentOperationalState, ComponentRecord, ReactionWheelState,
    VehicleComponentEntry, VehicleControlInput, VehiclePhysicalState, VehicleSnapshot,
};
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::numerical_integration::rk4_step;
use rocketcon_core::math::orbital::n_body::dop853::Dop853DenseSegment;
use rocketcon_core::math::{
    RigidBodyState, gimbal_actuator_step, reaction_wheel_torque_and_momentum_delta,
};
use rocketcon_db::repositories::{
    operational_state as operational_state_repository,
    reaction_wheel_state as reaction_wheel_state_repository, vehicle as vehicle_repository,
    vehicle_physical_state as vehicle_physical_state_repository,
};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) struct InitialAerodynamics<'a> {
    state: VehiclePhysicalState,
    environment: &'a EnvironmentSnapshot,
    components: &'a [(VehicleComponentEntry, ComponentRecord)],
    snapshot: &'a VehicleSnapshot,
    operational_states: &'a HashMap<Uuid, Option<ComponentOperationalState>>,
    diagnostic: Option<AerodynamicDiagnostic>,
}

impl<'a> InitialAerodynamics<'a> {
    pub(crate) fn new(
        state: VehiclePhysicalState,
        environment: &'a EnvironmentSnapshot,
        components: &'a [(VehicleComponentEntry, ComponentRecord)],
        snapshot: &'a VehicleSnapshot,
        operational_states: &'a HashMap<Uuid, Option<ComponentOperationalState>>,
        diagnostic: Option<AerodynamicDiagnostic>,
    ) -> Self {
        Self {
            state,
            environment,
            components,
            snapshot,
            operational_states,
            diagnostic,
        }
    }
}

pub async fn advance_vehicle_physical_state(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
) -> RocketResult<VehiclePhysicalState> {
    advance_vehicle_physical_state_inner(
        pool,
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        None,
        None,
        false,
        None,
        &mut Vec::new(),
    )
    .await
}

pub(crate) async fn advance_vehicle_physical_state_with_initial_aerodynamics(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
    initial_aerodynamics: InitialAerodynamics<'_>,
    effective_engine_loads: &HashMap<Uuid, f64>,
    nbody_catalog: Option<(&EphemerisCatalog, &[Uuid])>,
    position_segments: &mut Vec<Dop853DenseSegment<3>>,
) -> RocketResult<VehiclePhysicalState> {
    advance_vehicle_physical_state_inner(
        pool,
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        Some(initial_aerodynamics),
        Some(effective_engine_loads),
        true,
        nbody_catalog,
        position_segments,
    )
    .await
}

async fn advance_vehicle_physical_state_inner(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
    initial_aerodynamics: Option<InitialAerodynamics<'_>>,
    effective_engine_loads: Option<&HashMap<Uuid, f64>>,
    existing_transaction: bool,
    nbody_catalog: Option<(&EphemerisCatalog, &[Uuid])>,
    position_segments: &mut Vec<Dop853DenseSegment<3>>,
) -> RocketResult<VehiclePhysicalState> {
    let physical_state = if let Some(initial) = initial_aerodynamics.as_ref() {
        initial.state
    } else {
        vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
            .await?
            .ok_or_else(|| {
                RocketError::Generic(format!(
                    "physical state for vehicle '{}' not found",
                    vehicle_id
                ))
            })?
    };
    let current_at_epoch = physical_state.captured_at_epoch();
    let new_at_epoch = current_at_epoch + dt;
    let reference_body_id = physical_state.reference_body_id();

    let owned_components;
    let components: &[(VehicleComponentEntry, ComponentRecord)] =
        if let Some(initial) = initial_aerodynamics.as_ref() {
            initial.components
        } else {
            owned_components =
                vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
            &owned_components
        };
    let owned_snapshot;
    let snapshot: &VehicleSnapshot = if let Some(initial) = initial_aerodynamics.as_ref() {
        initial.snapshot
    } else {
        owned_snapshot =
            resolve_vehicle_snapshot(pool, vehicle_id, universe_epoch, current_at_epoch).await?;
        &owned_snapshot
    };
    let owned_environment;
    let environment: &EnvironmentSnapshot = if let Some(initial) = initial_aerodynamics.as_ref() {
        initial.environment
    } else {
        owned_environment =
            load_environment_snapshot(pool, reference_body_id, universe_epoch, current_at_epoch)
                .await?;
        &owned_environment
    };

    let mut operational_states = HashMap::new();
    for (entry, _) in components {
        let op = if let Some(&cached) = initial_aerodynamics
            .as_ref()
            .and_then(|initial| initial.operational_states.get(&entry.id()))
        {
            cached
        } else {
            operational_state_repository::get_by_vehicle_component_id(pool, &entry.id()).await?
        };
        if let Some(op) = op {
            operational_states.insert(entry.id(), op);
        }
    }

    let mut reaction_wheel_states = HashMap::new();
    for (entry, record) in components {
        if matches!(record.details(), ComponentDetails::ReactionWheel(_)) {
            if let Some(rw) =
                reaction_wheel_state_repository::get_by_vehicle_component_id(pool, &entry.id())
                    .await?
            {
                reaction_wheel_states.insert(entry.id(), rw);
            }
        }
    }

    for (entry, record) in components {
        if !snapshot.is_stage_active(entry.stage_index()) {
            continue;
        }

        if let ComponentDetails::Engine(engine) = record.details() {
            if let (Some(slew_rate), Some(cmd)) = (
                engine.gimbal_slew_rate(),
                control_input
                    .command_for(&entry.id())
                    .or_else(|| control_input.command_for(&entry.component_id())),
            ) {
                if let (Some(target_pitch), Some(target_yaw)) =
                    (cmd.target_gimbal_pitch, cmd.target_gimbal_yaw)
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
                        dt,
                    );

                    let updated_op = ComponentOperationalState::new(
                        entry.id(),
                        current_load,
                        Some(new_pitch),
                        Some(new_yaw),
                        universe_epoch,
                        new_at_epoch,
                    )?;
                    operational_state_repository::upsert(pool, &updated_op).await?;
                    operational_states.insert(entry.id(), updated_op);
                }
            }
        }
    }

    if let Some(loads) = effective_engine_loads {
        for (id, load) in loads {
            let previous = operational_states.get(id).copied();
            let effective = ComponentOperationalState::new(
                *id,
                *load,
                previous.and_then(|state| state.current_gimbal_pitch()),
                previous.and_then(|state| state.current_gimbal_yaw()),
                universe_epoch,
                current_at_epoch,
            )?;
            operational_states.insert(*id, effective);
        }
    }

    for (entry, record) in components {
        if !snapshot.is_stage_active(entry.stage_index()) {
            continue;
        }

        if let ComponentDetails::ReactionWheel(rw) = record.details() {
            let axis = entry
                .actuation_axis()
                .unwrap_or(Vector3::new(0.0, 0.0, 1.0))
                .normalized();
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
                dt,
            );

            let updated_rw =
                ReactionWheelState::new(entry.id(), new_momentum, universe_epoch, new_at_epoch)?;
            reaction_wheel_state_repository::upsert(pool, &updated_rw).await?;
            reaction_wheel_states.insert(entry.id(), updated_rw);
        }
    }

    let gravity_field = if nbody_catalog.is_none() {
        Some(GravityField::load(pool, environment, universe_epoch + current_at_epoch).await?)
    } else {
        None
    };

    let aero_diag = match initial_aerodynamics.as_ref() {
        Some(initial) => initial.diagnostic,
        _ if existing_transaction => {
            resolve_vehicle_aerodynamics_in_existing_transaction(
                pool,
                &physical_state,
                reference_body_id,
                environment.reference_position().raw(),
                components,
                snapshot.active_stages(),
                universe_epoch,
                current_at_epoch,
                None,
                None,
            )
            .await?
        }
        _ => {
            resolve_vehicle_aerodynamics(
                pool,
                &physical_state,
                reference_body_id,
                environment.reference_position().raw(),
                components,
                snapshot.active_stages(),
                universe_epoch,
                current_at_epoch,
            )
            .await?
        }
    };

    let (drag_force, ambient_pressure) = match aero_diag {
        Some(d) => (d.drag_force, d.ambient_pressure),
        None => (ForceVector::zero(), Pressure::new(0.0)),
    };

    let initial_rigid_state = RigidBodyState::from_physical_state(&physical_state);

    let new_rigid_state = if let Some((catalog, selected_ids)) = nbody_catalog {
        let mut gravity = CatalogGravityField::new(
            catalog,
            physical_state.captured_total_epoch(),
            initial_rigid_state.position.raw(),
            Some(selected_ids),
        )?;
        let impact = CatalogImpactMonitor::new(
            catalog,
            physical_state.captured_total_epoch(),
            initial_rigid_state.position.raw(),
            gravity.impact_candidates(),
        );
        integrate_nbody_rigid_body(
            &initial_rigid_state,
            dt,
            |t, sub_state| {
                let acceleration = gravity.acceleration_at(t, sub_state.position.raw())?;
                Ok(evaluate_rigid_body_derivative_with_ambient_pressure(
                    sub_state,
                    snapshot.mass_properties(),
                    components,
                    snapshot.active_stages(),
                    &operational_states,
                    &reaction_wheel_states,
                    control_input,
                    acceleration,
                    drag_force,
                    dt,
                    ambient_pressure,
                ))
            },
            |segment| Ok(impact.first_in(segment)?.map(|event| event.elapsed_seconds)),
            position_segments,
        )?
    } else {
        let field = gravity_field
            .as_ref()
            .ok_or_else(|| RocketError::Generic("legacy gravity field unavailable".into()))?;
        rk4_step(&initial_rigid_state, dt, |sub_state| {
            evaluate_rigid_body_derivative_with_ambient_pressure(
                sub_state,
                snapshot.mass_properties(),
                components,
                snapshot.active_stages(),
                &operational_states,
                &reaction_wheel_states,
                control_input,
                field.acceleration_at(sub_state.position),
                drag_force,
                dt,
                ambient_pressure,
            )
        })
    };

    let (max_q, max_q_epoch) = resolve_max_q(
        &physical_state,
        aero_diag.map(|diag| diag.dynamic_pressure),
        universe_epoch + current_at_epoch,
    );

    let new_physical_state = VehiclePhysicalState::new_with_max_q(
        vehicle_id,
        new_rigid_state.position,
        new_rigid_state.velocity,
        new_rigid_state.orientation,
        new_rigid_state.angular_velocity,
        reference_body_id,
        universe_epoch,
        new_at_epoch,
        max_q,
        max_q_epoch,
    )?;

    vehicle_physical_state_repository::upsert(pool, &new_physical_state).await?;

    Ok(new_physical_state)
}
