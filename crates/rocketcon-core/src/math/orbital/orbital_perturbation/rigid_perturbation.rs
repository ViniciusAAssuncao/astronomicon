use super::environment::PerturbedEnvironment;
use crate::math::propulsion_dynamics::VehiclePropulsionForces;
use crate::math::rigid_body_state::{RigidBodyDerivative, RigidBodyState};
use astronomicon_core::units::{
    AccelerationVector, AngularAccelerationVector, AngularVelocityVector, ForceVector,
    InertiaTensor, Mass, TorqueVector, Vector3,
};

pub fn angular_acceleration_from_torque(
    net_torque_body: TorqueVector,
    angular_velocity_body: AngularVelocityVector,
    inertia_tensor_body: &InertiaTensor,
) -> AngularAccelerationVector {
    let i_mat = inertia_tensor_body.matrix();
    let Some(i_inv) = inertia_tensor_body.inverse() else {
        return AngularAccelerationVector::zero();
    };

    let w = angular_velocity_body.raw();
    let h = i_mat.multiply_vector(w);
    let gyro_torque = w.cross(&h);
    let tau_eff = net_torque_body.raw() - gyro_torque;

    let alpha = i_inv.matrix().multiply_vector(tau_eff);
    if !alpha.0.is_finite() || !alpha.1.is_finite() || !alpha.2.is_finite() {
        AngularAccelerationVector::zero()
    } else {
        AngularAccelerationVector::from_components(alpha.0, alpha.1, alpha.2)
    }
}

pub fn compute_powered_flight_derivative(
    state: &RigidBodyState,
    total_mass: Mass,
    inertia_tensor: &InertiaTensor,
    gravity_acceleration: AccelerationVector,
    propulsion_forces: &VehiclePropulsionForces,
    aerodynamic_force: ForceVector,
) -> RigidBodyDerivative {
    let m = total_mass.value();
    let total_non_grav_force = propulsion_forces.net_world_force.raw() + aerodynamic_force.raw();

    let accel_non_grav = if m > 0.0 && m.is_finite() {
        total_non_grav_force / m
    } else {
        Vector3::zero()
    };

    let total_accel = gravity_acceleration.raw() + accel_non_grav;
    let alpha = angular_acceleration_from_torque(
        propulsion_forces.net_body_torque,
        state.angular_velocity,
        inertia_tensor,
    );

    RigidBodyDerivative::new(
        state.velocity,
        AccelerationVector::from_raw(total_accel),
        state.angular_velocity,
        alpha,
    )
}

pub fn evaluate_powered_flight_state_derivative(
    state: &RigidBodyState,
    total_mass: Mass,
    inertia_tensor: &InertiaTensor,
    environment: &PerturbedEnvironment,
    propulsion_forces: &VehiclePropulsionForces,
    aerodynamic_force: ForceVector,
) -> RigidBodyDerivative {
    let a_grav = environment.gravitational_acceleration_at(state.position);
    compute_powered_flight_derivative(
        state,
        total_mass,
        inertia_tensor,
        a_grav,
        propulsion_forces,
        aerodynamic_force,
    )
}
