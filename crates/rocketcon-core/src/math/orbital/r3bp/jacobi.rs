use super::equations::effective_potential;
use super::types::{Cr3bpParameters, SynodicState};
use astronomicon_core::units::Vector3;

pub fn compute_jacobi_constant(pos: Vector3, vel: Vector3, mu: f64) -> f64 {
    let omega = effective_potential(pos, mu);
    let v_sq = vel.dot(&vel);
    2.0 * omega - v_sq
}

pub fn jacobi_constant_from_state(state: &SynodicState, params: &Cr3bpParameters) -> f64 {
    compute_jacobi_constant(state.position, state.velocity, params.mu)
}

pub fn zero_velocity_potential(pos: Vector3, mu: f64) -> f64 {
    2.0 * effective_potential(pos, mu)
}

pub fn is_state_kinematically_admissible(pos: Vector3, jacobi_constant: f64, mu: f64) -> bool {
    let c_zv = zero_velocity_potential(pos, mu);
    c_zv >= jacobi_constant
}