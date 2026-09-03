use astronomicon_core::units::{
    AccelerationVector, AngularAccelerationVector, Density, Duration, ForceVector, Pressure,
    Speed, Vector3,
};
use rocketcon_core::domain::{
    ComponentOperationalState, ComponentRecord, ReactionWheelState, VehicleComponentEntry,
    VehicleControlInput,
};
use rocketcon_core::math::aerodynamics::{
    center_of_pressure, compute_aerodynamic_forces_and_torque, dynamic_pressure, mach_number,
    vehicle_reference_cross_section_area,
};
use rocketcon_core::math::{
    aggregate_active_thrust_and_torque_with_ambient_pressure, build_rcs_allocation_matrix,
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
    evaluate_rigid_body_derivative_with_ambient_pressure(
        state,
        mass_properties,
        components,
        active_stages,
        operational_states,
        reaction_wheel_states,
        control_input,
        gravitational_acceleration,
        aerodynamic_drag_force,
        dt,
        Pressure::new(0.0),
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
    let com = mass_properties.center_of_mass();
    let cop = center_of_pressure(components, active_stages, 1.0);
    let lever_arm = cop - com;
    let aero_force_body = state.orientation.inverse().rotate_vector(aerodynamic_drag_force.raw());
    let aero_torque_body = lever_arm.cross(&aero_force_body);

    let effective_control_input = if control_input.has_attitude_demand() {
        let rcs_matrix = build_rcs_allocation_matrix(
            components,
            active_stages,
            com,
        );
        let allocated_throttles = rcs_matrix.allocate_throttles(control_input);
        let mut updated_input = control_input.clone();
        for (entry_id, throttle) in allocated_throttles {
            if updated_input
                .command_for(&entry_id)
                .and_then(|c| c.target_rcs_throttle)
                .is_none()
            {
                let existing = updated_input.command_for(&entry_id).copied().unwrap_or_default();
                updated_input.actuator_commands.insert(
                    entry_id,
                    existing.with_rcs_throttle(throttle),
                );
            }
        }
        updated_input
    } else {
        control_input.clone()
    };

    let propulsion_forces = aggregate_active_thrust_and_torque_with_ambient_pressure(
        components,
        active_stages,
        mass_properties,
        operational_states,
        reaction_wheel_states,
        state.orientation,
        &effective_control_input,
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

    let tau_body_raw = propulsion_forces.net_body_torque.raw() + aero_torque_body;
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

pub fn evaluate_rigid_body_derivative_with_aero(
    state: &RigidBodyState,
    mass_properties: &MassProperties,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    operational_states: &HashMap<Uuid, ComponentOperationalState>,
    reaction_wheel_states: &HashMap<Uuid, ReactionWheelState>,
    control_input: &VehicleControlInput,
    gravitational_acceleration: AccelerationVector,
    air_density: Density,
    speed_of_sound: Speed,
    v_atm_inertial: Vector3,
    dt: Duration,
    ambient_pressure: Pressure,
) -> RigidBodyDerivative {
    let v_rel = state.velocity.raw() - v_atm_inertial;
    let v_rel_speed = Speed::new(v_rel.magnitude());
    let mach = mach_number(v_rel_speed, speed_of_sound);
    let q = dynamic_pressure(air_density, v_rel_speed);
    let s_ref = vehicle_reference_cross_section_area(components, active_stages);
    let cop = center_of_pressure(components, active_stages, mach);
    let com = mass_properties.center_of_mass();

    let (aero_force_world, aero_torque_body) = compute_aerodynamic_forces_and_torque(
        q,
        s_ref,
        mach,
        state.orientation,
        v_rel,
        cop,
        com,
    );

    let effective_control_input = if control_input.has_attitude_demand() {
        let rcs_matrix = build_rcs_allocation_matrix(
            components,
            active_stages,
            com,
        );
        let allocated_throttles = rcs_matrix.allocate_throttles(control_input);
        let mut updated_input = control_input.clone();
        for (entry_id, throttle) in allocated_throttles {
            if updated_input
                .command_for(&entry_id)
                .and_then(|c| c.target_rcs_throttle)
                .is_none()
            {
                let existing = updated_input.command_for(&entry_id).copied().unwrap_or_default();
                updated_input.actuator_commands.insert(
                    entry_id,
                    existing.with_rcs_throttle(throttle),
                );
            }
        }
        updated_input
    } else {
        control_input.clone()
    };

    let propulsion_forces = aggregate_active_thrust_and_torque_with_ambient_pressure(
        components,
        active_stages,
        mass_properties,
        operational_states,
        reaction_wheel_states,
        state.orientation,
        &effective_control_input,
        dt,
        ambient_pressure,
    );

    let m = mass_properties.total_mass().value();
    let grav_acc_raw = gravitational_acceleration.raw();
    let prop_force_raw = propulsion_forces.net_world_force.raw();
    let aero_force_raw = aero_force_world.raw();

    let net_linear_acceleration = if m > 0.0 && m.is_finite() {
        grav_acc_raw + (prop_force_raw + aero_force_raw) / m
    } else {
        grav_acc_raw
    };

    let tau_body_raw = propulsion_forces.net_body_torque.raw() + aero_torque_body.raw();
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
