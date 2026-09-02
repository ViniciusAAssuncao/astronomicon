use crate::domain::{
    ComponentDetails,
    ComponentOperationalState,
    ComponentRecord,
    EngineSpecification,
    ReactionControlThrusterSpecification,
    ReactionWheelSpecification,
    ReactionWheelState,
    VehicleComponentEntry,
    VehicleControlInput,
};
use crate::math::MassProperties;
use astronomicon_core::units::{
    Angle,
    AngularMomentum,
    AngularVelocity,
    Duration,
    ForceVector,
    Quaternion,
    TorqueVector,
    Vector3,
};
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use uuid::Uuid;

pub fn engine_thrust_direction_local(
    neutral_axis: Vector3,
    gimbal_pitch: Angle,
    gimbal_yaw: Angle
) -> Vector3 {
    let n = neutral_axis.normalized();
    if n.magnitude() < 1e-12 {
        return Vector3::new(0.0, 0.0, 1.0);
    }
    let p1 = n.any_perpendicular().normalized();
    let p2 = n.cross(&p1).normalized();
    let d1 = n.rotate_about_axis(p1, gimbal_pitch.value());
    d1.rotate_about_axis(p2, gimbal_yaw.value()).normalized()
}

pub fn engine_thrust_direction_local_clamped(
    neutral_axis: Vector3,
    gimbal_pitch: Angle,
    gimbal_yaw: Angle,
    max_gimbal_deflection: Option<Angle>
) -> Vector3 {
    let (pitch_rad, yaw_rad) = match max_gimbal_deflection {
        Some(max_def) => {
            let max_val = max_def.value().abs();
            (
                gimbal_pitch.value().clamp(-max_val, max_val),
                gimbal_yaw.value().clamp(-max_val, max_val),
            )
        }
        None => (gimbal_pitch.value(), gimbal_yaw.value()),
    };
    engine_thrust_direction_local(neutral_axis, Angle::new(pitch_rad), Angle::new(yaw_rad))
}

pub fn engine_thrust_force(
    spec: &EngineSpecification,
    load_fraction: f64,
    thrust_direction_world: Vector3
) -> ForceVector {
    let load = load_fraction.clamp(0.0, 1.0);
    let mag = spec.max_thrust().value() * load;
    let dir = thrust_direction_world.normalized();
    ForceVector::from_raw(dir * mag)
}

pub fn rcs_thrust_force(
    spec: &ReactionControlThrusterSpecification,
    load_fraction: f64,
    thrust_direction_world: Vector3
) -> ForceVector {
    let load = load_fraction.clamp(0.0, 1.0);
    let mag = spec.max_thrust().value() * load;
    let dir = thrust_direction_world.normalized();
    ForceVector::from_raw(dir * mag)
}

pub fn reaction_wheel_torque_and_momentum_delta(
    spec: &ReactionWheelSpecification,
    commanded_torque_fraction: f64,
    axis: Vector3,
    current_stored_momentum: AngularMomentum,
    dt: Duration
) -> (TorqueVector, AngularMomentum) {
    let dt_s = dt.value();
    let max_torque_val = spec.max_torque().value();
    let max_momentum_val = spec.max_angular_momentum_storage().value();
    let current_h = current_stored_momentum.value();

    if dt_s <= 0.0 || !dt_s.is_finite() || max_torque_val <= 0.0 || max_momentum_val <= 0.0 {
        return (TorqueVector::zero(), current_stored_momentum);
    }

    let frac = commanded_torque_fraction.clamp(-1.0, 1.0);
    let requested_torque = max_torque_val * frac;
    let requested_delta_h = requested_torque * dt_s;

    let (actual_delta_h, new_h) = if requested_delta_h >= 0.0 {
        let max_deliverable_delta = (max_momentum_val - current_h).max(0.0);
        let actual_delta = requested_delta_h.min(max_deliverable_delta);
        (actual_delta, (current_h + actual_delta).min(max_momentum_val))
    } else {
        let min_deliverable_delta = (-max_momentum_val - current_h).min(0.0);
        let actual_delta = requested_delta_h.max(min_deliverable_delta);
        (actual_delta, (current_h + actual_delta).max(-max_momentum_val))
    };

    let actual_torque = actual_delta_h / dt_s;
    let norm_axis = axis.normalized();
    let torque_vec = TorqueVector::from_raw(norm_axis * actual_torque);

    (torque_vec, AngularMomentum::new(new_h))
}

pub fn gimbal_actuator_step(
    current_pitch: Angle,
    current_yaw: Angle,
    target_pitch: Angle,
    target_yaw: Angle,
    slew_rate: AngularVelocity,
    dt: Duration
) -> (Angle, Angle) {
    let dt_s = dt.value();
    let rate = slew_rate.value();

    if dt_s <= 0.0 || rate <= 0.0 || !dt_s.is_finite() || !rate.is_finite() {
        return (current_pitch, current_yaw);
    }

    let max_delta = rate * dt_s;

    let diff_pitch = target_pitch.value() - current_pitch.value();
    let step_pitch = diff_pitch.clamp(-max_delta, max_delta);
    let new_pitch = current_pitch.value() + step_pitch;

    let diff_yaw = target_yaw.value() - current_yaw.value();
    let step_yaw = diff_yaw.clamp(-max_delta, max_delta);
    let new_yaw = current_yaw.value() + step_yaw;

    (Angle::new(new_pitch), Angle::new(new_yaw))
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehiclePropulsionForces {
    pub net_world_force: ForceVector,
    pub net_body_torque: TorqueVector,
}

impl VehiclePropulsionForces {
    pub fn new(net_world_force: ForceVector, net_body_torque: TorqueVector) -> Self {
        Self {
            net_world_force,
            net_body_torque,
        }
    }

    pub fn zero() -> Self {
        Self {
            net_world_force: ForceVector::zero(),
            net_body_torque: TorqueVector::zero(),
        }
    }
}

pub fn aggregate_active_thrust_and_torque(
    entries_and_records: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    mass_properties: &MassProperties,
    operational_states: &HashMap<Uuid, ComponentOperationalState>,
    reaction_wheel_states: &HashMap<Uuid, ReactionWheelState>,
    vehicle_orientation: Quaternion,
    control_input: &VehicleControlInput,
    dt: Duration
) -> VehiclePropulsionForces {
    let mut total_world_force = Vector3::zero();
    let mut total_body_torque = Vector3::zero();
    let com = mass_properties.center_of_mass();

    for (entry, record) in entries_and_records {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        let r_arm = entry.mount_offset() - com;

        match record.details() {
            ComponentDetails::Engine(engine) => {
                let op_state = operational_states
                    .get(&entry.id())
                    .or_else(|| operational_states.get(&entry.component_id()));
                let load_fraction = op_state.map(|s| s.load_fraction()).unwrap_or(1.0);

                let neutral_axis = entry.actuation_axis().unwrap_or(Vector3::new(0.0, 0.0, 1.0));

                let pitch = op_state
                    .and_then(|s| s.current_gimbal_pitch())
                    .unwrap_or(Angle::new(0.0));
                let yaw = op_state.and_then(|s| s.current_gimbal_yaw()).unwrap_or(Angle::new(0.0));

                let thrust_dir_body = engine_thrust_direction_local_clamped(
                    neutral_axis,
                    pitch,
                    yaw,
                    engine.max_gimbal_deflection()
                );

                let thrust_mag = engine.max_thrust().value() * load_fraction.clamp(0.0, 1.0);
                let force_body = thrust_dir_body * thrust_mag;
                let force_world = vehicle_orientation.rotate_vector(force_body);

                total_world_force = total_world_force + force_world;

                let torque_body = r_arm.cross(&force_body);
                total_body_torque = total_body_torque + torque_body;
            }
            ComponentDetails::ReactionControlThruster(rcs) => {
                let op_state = operational_states
                    .get(&entry.id())
                    .or_else(|| operational_states.get(&entry.component_id()));
                let load_fraction = op_state.map(|s| s.load_fraction()).unwrap_or(1.0);

                let thrust_axis = entry
                    .actuation_axis()
                    .unwrap_or(Vector3::new(0.0, 0.0, 1.0))
                    .normalized();

                let thrust_mag = rcs.max_thrust().value() * load_fraction.clamp(0.0, 1.0);
                let force_body = thrust_axis * thrust_mag;
                let force_world = vehicle_orientation.rotate_vector(force_body);

                total_world_force = total_world_force + force_world;

                let torque_body = r_arm.cross(&force_body);
                total_body_torque = total_body_torque + torque_body;
            }
            ComponentDetails::ReactionWheel(rw) => {
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

                let current_momentum = reaction_wheel_states
                    .get(&entry.id())
                    .or_else(|| reaction_wheel_states.get(&entry.component_id()))
                    .map(|s| s.stored_angular_momentum())
                    .unwrap_or(AngularMomentum::new(0.0));

                let (torque_vec, _) = reaction_wheel_torque_and_momentum_delta(
                    rw,
                    torque_frac,
                    axis,
                    current_momentum,
                    dt
                );

                total_body_torque = total_body_torque + torque_vec.raw();
            }
            _ => {}
        }
    }

    VehiclePropulsionForces::new(
        ForceVector::from_raw(total_world_force),
        TorqueVector::from_raw(total_body_torque)
    )
}
