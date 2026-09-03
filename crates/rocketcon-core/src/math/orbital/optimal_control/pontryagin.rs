use super::types::{ OptimalControlProblemType, OptimalControlState };
use astronomicon_core::units::Vector3;

pub fn primer_vector_optimal_thrust_direction(lambda_v: Vector3) -> Vector3 {
    let mag = lambda_v.magnitude();
    if mag > 1e-12 {
        lambda_v / mag
    } else {
        Vector3::new(1.0, 0.0, 0.0)
    }
}

pub fn switching_function(
    lambda_v: Vector3,
    mass: f64,
    lambda_m: f64,
    effective_exhaust_velocity: f64
) -> f64 {
    if mass <= 0.0 || effective_exhaust_velocity <= 0.0 {
        return 0.0;
    }
    lambda_v.magnitude() / mass - lambda_m / effective_exhaust_velocity
}

pub fn throttle_command(switching_val: f64, problem_type: OptimalControlProblemType) -> f64 {
    match problem_type {
        OptimalControlProblemType::MinimumTime => 1.0,
        OptimalControlProblemType::MinimumFuel => {
            if switching_val > 0.0 { 1.0 } else { 0.0 }
        }
        OptimalControlProblemType::EnergyOptimal { smoothing_epsilon } => {
            let eps = smoothing_epsilon.max(1e-4);
            0.5 * (1.0 + (switching_val / eps).tanh())
        }
    }
}

pub fn compute_hamiltonian(
    state: &OptimalControlState,
    mu: f64,
    thrust: f64,
    effective_exhaust_velocity: f64,
    problem_type: OptimalControlProblemType
) -> f64 {
    let r_mag = state.position.magnitude();
    if r_mag < 1e-6 || state.mass <= 0.0 {
        return 0.0;
    }

    let a_grav = -state.position * (mu / (r_mag * r_mag * r_mag));
    let sw = switching_function(
        state.lambda_v,
        state.mass,
        state.lambda_m,
        effective_exhaust_velocity
    );
    let u = throttle_command(sw, problem_type);
    let u_thrust = primer_vector_optimal_thrust_direction(state.lambda_v);
    let a_thrust = u_thrust * ((thrust * u) / state.mass);

    let h_kin = state.lambda_r.dot(&state.velocity);
    let h_acc = state.lambda_v.dot(&(a_grav + a_thrust));
    let h_mass = -state.lambda_m * ((thrust * u) / effective_exhaust_velocity);

    let l_term = match problem_type {
        OptimalControlProblemType::MinimumTime => 1.0,
        OptimalControlProblemType::MinimumFuel => (thrust * u) / effective_exhaust_velocity,
        OptimalControlProblemType::EnergyOptimal { .. } => 0.5 * u * u,
    };

    h_kin + h_acc + h_mass + l_term
}
