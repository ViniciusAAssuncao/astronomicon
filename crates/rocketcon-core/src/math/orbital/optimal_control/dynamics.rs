use super::pontryagin::{
    primer_vector_optimal_thrust_direction, switching_function, throttle_command,
};
use super::types::{OptimalControlDerivative, OptimalControlProblemType, OptimalControlState};
use astronomicon_core::units::Vector3;

pub fn evaluate_optimal_control_dynamics(
    state: &OptimalControlState,
    mu: f64,
    thrust: f64,
    effective_exhaust_velocity: f64,
    problem_type: OptimalControlProblemType,
) -> OptimalControlDerivative {
    let r_mag = state.position.magnitude();
    if r_mag < 1e-6 || state.mass <= 0.0 {
        return OptimalControlDerivative {
            d_position: state.velocity,
            d_velocity: Vector3::zero(),
            d_mass: 0.0,
            d_lambda_r: Vector3::zero(),
            d_lambda_v: Vector3::zero(),
            d_lambda_m: 0.0,
        };
    }

    let r_cubed = r_mag * r_mag * r_mag;
    let r_fifth = r_cubed * r_mag * r_mag;
    let a_grav = -state.position * (mu / r_cubed);

    let sw = switching_function(
        state.lambda_v,
        state.mass,
        state.lambda_m,
        effective_exhaust_velocity,
    );
    let u = throttle_command(sw, problem_type);
    let u_thrust = primer_vector_optimal_thrust_direction(state.lambda_v);
    let a_thrust = u_thrust * (thrust * u / state.mass);
    let d_vel = a_grav + a_thrust;

    let m_dot = -(thrust * u) / effective_exhaust_velocity;

    let r_dot_lv = state.position.dot(&state.lambda_v);
    let d_lr = state.lambda_v * (mu / r_cubed) - state.position * (3.0 * mu * r_dot_lv / r_fifth);
    let d_lv = -state.lambda_r;

    let lv_mag = state.lambda_v.magnitude();
    let d_lm = (thrust * u / (state.mass * state.mass)) * lv_mag;

    OptimalControlDerivative {
        d_position: state.velocity,
        d_velocity: d_vel,
        d_mass: m_dot,
        d_lambda_r: d_lr,
        d_lambda_v: d_lv,
        d_lambda_m: d_lm,
    }
}