use super::types::{Er3bpParameters, SynodicState};
use crate::constants::CR3BP_MIN_DISTANCE_SINGULARITY;
use astronomicon_core::units::Vector3;

pub fn primary_distance(pos: Vector3, mu: f64) -> f64 {
    let dx = pos.0 + mu;
    let dy = pos.1;
    let dz = pos.2;
    (dx * dx + dy * dy + dz * dz).max(CR3BP_MIN_DISTANCE_SINGULARITY).sqrt()
}

pub fn secondary_distance(pos: Vector3, mu: f64) -> f64 {
    let dx = pos.0 - (1.0 - mu);
    let dy = pos.1;
    let dz = pos.2;
    (dx * dx + dy * dy + dz * dz).max(CR3BP_MIN_DISTANCE_SINGULARITY).sqrt()
}

pub fn effective_potential(pos: Vector3, mu: f64) -> f64 {
    let x = pos.0;
    let y = pos.1;
    let r1 = primary_distance(pos, mu);
    let r2 = secondary_distance(pos, mu);
    0.5 * (x * x + y * y) + (1.0 - mu) / r1 + mu / r2
}

pub fn effective_potential_gradient(pos: Vector3, mu: f64) -> Vector3 {
    let x = pos.0;
    let y = pos.1;
    let z = pos.2;
    let r1 = primary_distance(pos, mu);
    let r2 = secondary_distance(pos, mu);
    let r1_3 = r1 * r1 * r1;
    let r2_3 = r2 * r2 * r2;

    let gx = x - ((1.0 - mu) * (x + mu)) / r1_3 - (mu * (x - (1.0 - mu))) / r2_3;
    let gy = y - ((1.0 - mu) * y) / r1_3 - (mu * y) / r2_3;
    let gz = -((1.0 - mu) * z) / r1_3 - (mu * z) / r2_3;

    Vector3::new(gx, gy, gz)
}

pub fn effective_potential_hessian(pos: Vector3, mu: f64) -> [[f64; 3]; 3] {
    let x = pos.0;
    let y = pos.1;
    let z = pos.2;
    let r1 = primary_distance(pos, mu);
    let r2 = secondary_distance(pos, mu);
    let r1_3 = r1 * r1 * r1;
    let r2_3 = r2 * r2 * r2;
    let r1_5 = r1_3 * r1 * r1;
    let r2_5 = r2_3 * r2 * r2;

    let dx1 = x + mu;
    let dx2 = x - (1.0 - mu);

    let g1 = (1.0 - mu) / r1_3;
    let g2 = mu / r2_3;

    let hxx = 1.0 - g1 - g2 + (3.0 * (1.0 - mu) * dx1 * dx1) / r1_5 + (3.0 * mu * dx2 * dx2) / r2_5;
    let hyy = 1.0 - g1 - g2 + (3.0 * (1.0 - mu) * y * y) / r1_5 + (3.0 * mu * y * y) / r2_5;
    let hzz = -g1 - g2 + (3.0 * (1.0 - mu) * z * z) / r1_5 + (3.0 * mu * z * z) / r2_5;

    let hxy = (3.0 * (1.0 - mu) * dx1 * y) / r1_5 + (3.0 * mu * dx2 * y) / r2_5;
    let hxz = (3.0 * (1.0 - mu) * dx1 * z) / r1_5 + (3.0 * mu * dx2 * z) / r2_5;
    let hyz = (3.0 * (1.0 - mu) * y * z) / r1_5 + (3.0 * mu * y * z) / r2_5;

    [
        [hxx, hxy, hxz],
        [hxy, hyy, hyz],
        [hxz, hyz, hzz],
    ]
}

pub fn cr3bp_acceleration(pos: Vector3, vel: Vector3, mu: f64) -> Vector3 {
    let grad = effective_potential_gradient(pos, mu);
    let ax = 2.0 * vel.1 + grad.0;
    let ay = -2.0 * vel.0 + grad.1;
    let az = grad.2;
    Vector3::new(ax, ay, az)
}

pub fn cr3bp_derivative(state: &SynodicState, mu: f64) -> SynodicState {
    let acc = cr3bp_acceleration(state.position, state.velocity, mu);
    SynodicState::new(state.velocity, acc)
}

pub fn er3bp_acceleration(pos: Vector3, vel: Vector3, true_anomaly: f64, er3bp: &Er3bpParameters) -> Vector3 {
    let e = er3bp.eccentricity;
    let mu = er3bp.cr3bp.mu;
    let cos_nu = true_anomaly.cos();
    let f_nu = 1.0 / (1.0 + e * cos_nu).max(1e-6);

    let grad = effective_potential_gradient(pos, mu);
    let ax = 2.0 * vel.1 + f_nu * grad.0;
    let ay = -2.0 * vel.0 + f_nu * grad.1;
    let az = -pos.2 + f_nu * (grad.2 + pos.2);

    Vector3::new(ax, ay, az)
}