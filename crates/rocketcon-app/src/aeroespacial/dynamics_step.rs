use astronomicon_core::units::{
    AccelerationVector, AngularAccelerationVector, Duration, ForceVector, Pressure, Vector3,
};
use rocketcon_core::domain::{
    ComponentOperationalState, ComponentRecord, ReactionWheelState, VehicleComponentEntry,
    VehicleControlInput,
};
use rocketcon_core::math::{
    aggregate_active_thrust_and_torque, aggregate_active_thrust_and_torque_with_ambient_pressure,
    MassProperties, RigidBodyDerivative, RigidBodyState,
};
use std::collections::HashMap;
use uuid::Uuid;

pub fn evaluate_rigid_body_derivative(
    state: &RigidBodyState,
    mass_properties: &MassProperties,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    operational_states: &HashMap<Uuid, ComponentOperationalState>,
    reaction_wheel_states: &HashMap<Uuid, ReactionWheelState>,
    control_input: &VehicleControlInput,
    gravitational_acceleration: AccelerationVector,
    aerodynamic_drag_force: ForceVector,
    dt: Duration,
) -> RigidBodyDerivative {
    let propulsion_forces = aggregate_active_thrust_and_torque(
        components,
        active_stages,
        mass_properties,
        operational_states,
        reaction_wheel_states,
        state.orientation,
        control_input,
        dt,
    );

    let m = mass_properties.total_mass().value();
    let grav_acc_raw = gravitational_acceleration.raw();
    let prop_force_raw = propulsion_forces.net_world_force.raw();
    let aero_drag_raw = aerodynamic_drag_force.raw();

    let net_linear_acceleration = if m > 0.0 && m.is_finite() {
        grav_acc_raw + (prop_force_raw + aero_drag_raw) / m
    } else {
        grav_acc_raw
    };

    let tau_body_raw = propulsion_forces.net_body_torque.raw();
    let omega_raw = state.angular_velocity.raw();
    let inertia = mass_properties.inertia_tensor();

    let i_omega = inertia.raw().multiply_vector(omega_raw);
    let gyroscopic_torque = omega_raw.cross(&i_omega);
    let net_torque = tau_body_raw - gyroscopic_torque;

    let angular_acc_raw = match inertia.inverse() {
        Some(i_inv) => i_inv.raw().multiply_vector(net_torque),
        None => Vector3::zero(),
    };

    RigidBodyDerivative::new(
        state.velocity,
        AccelerationVector::from_raw(net_linear_acceleration),
        state.angular_velocity,
        AngularAccelerationVector::from_raw(angular_acc_raw),
    )
}

pub fn evaluate_rigid_body_derivative_with_ambient_pressure(
    state: &RigidBodyState,
    mass_properties: &MassProperties,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    operational_states: &HashMap<Uuid, ComponentOperationalState>,
    reaction_wheel_states: &HashMap<Uuid, ReactionWheelState>,
    control_input: &VehicleControlInput,
    gravitational_acceleration: AccelerationVector,
    aerodynamic_drag_force: ForceVector,
    dt: Duration,
    ambient_pressure: Pressure,
) -> RigidBodyDerivative {
    let propulsion_forces = aggregate_active_thrust_and_torque_with_ambient_pressure(
        components,
        active_stages,
        mass_properties,
        operational_states,
        reaction_wheel_states,
        state.orientation,
        control_input,
        dt,
        ambient_pressure,
    );

    let m = mass_properties.total_mass().value();
    let grav_acc_raw = gravitational_acceleration.raw();
    let prop_force_raw = propulsion_forces.net_world_force.raw();
    let aero_drag_raw = aerodynamic_drag_force.raw();

    let net_linear_acceleration = if m > 0.0 && m.is_finite() {
        grav_acc_raw + (prop_force_raw + aero_drag_raw) / m
    } else {
        grav_acc_raw
    };

    let tau_body_raw = propulsion_forces.net_body_torque.raw();
    let omega_raw = state.angular_velocity.raw();
    let inertia = mass_properties.inertia_tensor();

    let i_omega = inertia.raw().multiply_vector(omega_raw);
    let gyroscopic_torque = omega_raw.cross(&i_omega);
    let net_torque = tau_body_raw - gyroscopic_torque;

    let angular_acc_raw = match inertia.inverse() {
        Some(i_inv) => i_inv.raw().multiply_vector(net_torque),
        None => Vector3::zero(),
    };

    RigidBodyDerivative::new(
        state.velocity,
        AccelerationVector::from_raw(net_linear_acceleration),
        state.angular_velocity,
        AngularAccelerationVector::from_raw(angular_acc_raw),
    )
}
