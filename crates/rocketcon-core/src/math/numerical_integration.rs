use crate::orbital::orbital_perturbation::{
    evaluate_powered_flight_state_derivative, PerturbedEnvironment,
};
use crate::math::propulsion_dynamics::VehiclePropulsionForces;
use crate::math::rigid_body_state::{RigidBodyDerivative, RigidBodyState};
use astronomicon_core::units::{
    AngularVelocityVector, Duration, ForceVector, InertiaTensor, Mass, Position, Quaternion,
    VelocityVector,
};

pub fn rk4_step<F>(state: &RigidBodyState, dt: Duration, mut derivative_fn: F) -> RigidBodyState
where
    F: FnMut(&RigidBodyState) -> RigidBodyDerivative,
{
    let h = dt.value();
    if h <= 0.0 || !h.is_finite() {
        return *state;
    }

    let half_h = 0.5 * h;
    let sixth_h = h / 6.0;

    let d1 = derivative_fn(state);
    let k1_r = d1.velocity.raw();
    let k1_v = d1.acceleration.raw();
    let k1_q = state.orientation.derivative(d1.angular_velocity);
    let k1_w = d1.angular_acceleration.raw();

    let s1 = RigidBodyState::new(
        Position::from_raw(state.position.raw() + k1_r * half_h),
        VelocityVector::from_raw(state.velocity.raw() + k1_v * half_h),
        state.orientation.add_scaled(k1_q, half_h),
        AngularVelocityVector::from_raw(state.angular_velocity.raw() + k1_w * half_h),
    );

    let d2 = derivative_fn(&s1);
    let k2_r = d2.velocity.raw();
    let k2_v = d2.acceleration.raw();
    let k2_q = s1.orientation.derivative(d2.angular_velocity);
    let k2_w = d2.angular_acceleration.raw();

    let s2 = RigidBodyState::new(
        Position::from_raw(state.position.raw() + k2_r * half_h),
        VelocityVector::from_raw(state.velocity.raw() + k2_v * half_h),
        state.orientation.add_scaled(k2_q, half_h),
        AngularVelocityVector::from_raw(state.angular_velocity.raw() + k2_w * half_h),
    );

    let d3 = derivative_fn(&s2);
    let k3_r = d3.velocity.raw();
    let k3_v = d3.acceleration.raw();
    let k3_q = s2.orientation.derivative(d3.angular_velocity);
    let k3_w = d3.angular_acceleration.raw();

    let s3 = RigidBodyState::new(
        Position::from_raw(state.position.raw() + k3_r * h),
        VelocityVector::from_raw(state.velocity.raw() + k3_v * h),
        state.orientation.add_scaled(k3_q, h),
        AngularVelocityVector::from_raw(state.angular_velocity.raw() + k3_w * h),
    );

    let d4 = derivative_fn(&s3);
    let k4_r = d4.velocity.raw();
    let k4_v = d4.acceleration.raw();
    let k4_q = s3.orientation.derivative(d4.angular_velocity);
    let k4_w = d4.angular_acceleration.raw();

    let r_final = Position::from_raw(
        state.position.raw() + (k1_r + k2_r * 2.0 + k3_r * 2.0 + k4_r) * sixth_h,
    );
    let v_final = VelocityVector::from_raw(
        state.velocity.raw() + (k1_v + k2_v * 2.0 + k3_v * 2.0 + k4_v) * sixth_h,
    );

    let q_dot_comb = Quaternion::new(
        k1_q.w() + 2.0 * k2_q.w() + 2.0 * k3_q.w() + k4_q.w(),
        k1_q.x() + 2.0 * k2_q.x() + 2.0 * k3_q.x() + k4_q.x(),
        k1_q.y() + 2.0 * k2_q.y() + 2.0 * k3_q.y() + k4_q.y(),
        k1_q.z() + 2.0 * k2_q.z() + 2.0 * k3_q.z() + k4_q.z(),
    );
    let q_final = state.orientation.add_scaled(q_dot_comb, sixth_h).normalized();

    let w_final = AngularVelocityVector::from_raw(
        state.angular_velocity.raw() + (k1_w + k2_w * 2.0 + k3_w * 2.0 + k4_w) * sixth_h,
    );

    RigidBodyState::new(r_final, v_final, q_final, w_final)
}

pub fn integrate_substeps<F>(
    state: &RigidBodyState,
    total_duration: Duration,
    substep_duration: Duration,
    mut derivative_fn: F,
) -> RigidBodyState
where
    F: FnMut(&RigidBodyState) -> RigidBodyDerivative,
{
    let total_dt = total_duration.value();
    let sub_dt = substep_duration.value();

    if total_dt <= 0.0 || !total_dt.is_finite() {
        return *state;
    }

    if sub_dt <= 0.0 || !sub_dt.is_finite() || sub_dt >= total_dt {
        return rk4_step(state, total_duration, derivative_fn);
    }

    let mut current = *state;
    let mut remaining = total_dt;

    while remaining > 1e-12 {
        let step = remaining.min(sub_dt);
        current = rk4_step(&current, Duration::new(step), &mut derivative_fn);
        remaining -= step;
    }

    current
}

pub fn integrate_powered_flight_step(
    state: &RigidBodyState,
    total_mass: Mass,
    inertia_tensor: &InertiaTensor,
    environment: &PerturbedEnvironment,
    propulsion_forces: &VehiclePropulsionForces,
    aerodynamic_force: ForceVector,
    dt: Duration,
) -> RigidBodyState {
    rk4_step(state, dt, |s| {
        evaluate_powered_flight_state_derivative(
            s,
            total_mass,
            inertia_tensor,
            environment,
            propulsion_forces,
            aerodynamic_force,
        )
    })
}