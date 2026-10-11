use super::cowell::cowell_acceleration;
use super::types::CowellPerturbationConfig;
use crate::constants::{
    COWELL_MAX_SCALE_FACTOR, COWELL_MAX_TIME_STEP_S, COWELL_MIN_SCALE_FACTOR,
    COWELL_MIN_TIME_STEP_S, COWELL_SAFETY_FACTOR,
};
use astronomicon_core::units::Vector3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dp853State {
    pub position: Vector3,
    pub velocity: Vector3,
}

impl Dp853State {
    pub fn new(position: Vector3, velocity: Vector3) -> Self {
        Self { position, velocity }
    }
}

pub fn dp853_step(
    state: &Dp853State,
    config: &CowellPerturbationConfig,
    dt: f64,
) -> (Dp853State, f64) {
    let p0 = state.position;
    let v0 = state.velocity;

    let f = |p: Vector3, v: Vector3| -> (Vector3, Vector3) {
        let a = cowell_acceleration(p, config);
        (v, a)
    };

    let (v1, a1) = f(p0, v0);

    let k1_p = v1 * dt;
    let k1_v = a1 * dt;

    let (v2, a2) = f(p0 + k1_p * (2.0 / 27.0), v0 + k1_v * (2.0 / 27.0));
    let k2_p = v2 * dt;
    let k2_v = a2 * dt;

    let (v3, a3) = f(
        p0 + k1_p * (1.0 / 36.0) + k2_p * (1.0 / 12.0),
        v0 + k1_v * (1.0 / 36.0) + k2_v * (1.0 / 12.0),
    );
    let k3_p = v3 * dt;
    let k3_v = a3 * dt;

    let (v4, a4) = f(
        p0 + k1_p * (1.0 / 24.0) + k3_p * (1.0 / 8.0),
        v0 + k1_v * (1.0 / 24.0) + k3_v * (1.0 / 8.0),
    );
    let k4_p = v4 * dt;
    let k4_v = a4 * dt;

    let (v5, a5) = f(
        p0 + k1_p * (5.0 / 12.0) - k3_p * (25.0 / 16.0) + k4_p * (25.0 / 16.0),
        v0 + k1_v * (5.0 / 12.0) - k3_v * (25.0 / 16.0) + k4_v * (25.0 / 16.0),
    );
    let k5_p = v5 * dt;
    let k5_v = a5 * dt;

    let (v6, a6) = f(
        p0 + k1_p * (1.0 / 20.0) + k4_p * (1.0 / 4.0) + k5_p * (1.0 / 5.0),
        v0 + k1_v * (1.0 / 20.0) + k4_v * (1.0 / 4.0) + k5_v * (1.0 / 5.0),
    );
    let k6_p = v6 * dt;
    let k6_v = a6 * dt;

    let (v7, a7) = f(
        p0 - k1_p * (25.0 / 108.0) + k4_p * (125.0 / 108.0) - k5_p * (65.0 / 27.0) + k6_p * (125.0 / 54.0),
        v0 - k1_v * (25.0 / 108.0) + k4_v * (125.0 / 108.0) - k5_v * (65.0 / 27.0) + k6_v * (125.0 / 54.0),
    );
    let k7_p = v7 * dt;
    let k7_v = a7 * dt;

    let (v8, a8) = f(
        p0 + k1_p * (31.0 / 300.0) + k5_p * (61.0 / 225.0) - k6_p * (2.0 / 9.0) + k7_p * (13.0 / 900.0),
        v0 + k1_v * (31.0 / 300.0) + k5_v * (61.0 / 225.0) - k6_v * (2.0 / 9.0) + k7_v * (13.0 / 900.0),
    );
    let k8_p = v8 * dt;
    let k8_v = a8 * dt;

    let p_next = p0
        + k1_p * (23.0 / 192.0)
        + k5_p * (125.0 / 192.0)
        + k7_p * (-81.0 / 192.0)
        + k8_p * (125.0 / 192.0);

    let v_next = v0
        + k1_v * (23.0 / 192.0)
        + k5_v * (125.0 / 192.0)
        + k7_v * (-81.0 / 192.0)
        + k8_v * (125.0 / 192.0);

    let p_err = (k1_p * (-4.0 / 192.0)
        + k5_p * (25.0 / 192.0)
        + k7_p * (-33.0 / 192.0)
        + k8_p * (12.0 / 192.0))
        .magnitude();

    let v_err = (k1_v * (-4.0 / 192.0)
        + k5_v * (25.0 / 192.0)
        + k7_v * (-33.0 / 192.0)
        + k8_v * (12.0 / 192.0))
        .magnitude();

    let total_err = p_err + v_err * dt.max(1.0);
    (Dp853State::new(p_next, v_next), total_err)
}

pub fn adaptive_dp853_step(
    state: &Dp853State,
    config: &CowellPerturbationConfig,
    dt: f64,
    atol: f64,
    rtol: f64,
) -> (Dp853State, f64, bool) {
    let (next_state, error) = dp853_step(state, config, dt);
    let scale = atol + rtol * (state.position.magnitude() + state.velocity.magnitude() * dt);
    let err_ratio = (error / scale.max(1e-15)).max(1e-10);

    let factor = (COWELL_SAFETY_FACTOR * (1.0 / err_ratio).powf(0.125))
        .clamp(COWELL_MIN_SCALE_FACTOR, COWELL_MAX_SCALE_FACTOR);
    let next_dt = (dt * factor).clamp(COWELL_MIN_TIME_STEP_S, COWELL_MAX_TIME_STEP_S);

    if err_ratio <= 1.0 {
        (next_state, next_dt, true)
    } else {
        (*state, next_dt, false)
    }
}