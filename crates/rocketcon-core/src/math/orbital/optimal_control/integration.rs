use super::dynamics::evaluate_optimal_control_dynamics;
use super::pontryagin::{compute_hamiltonian, switching_function, throttle_command};
use super::types::{OptimalControlProblemType, OptimalControlSolution, OptimalControlState};
use astronomicon_core::units::{Duration, Mass, Speed};

pub fn integrate_optimal_control_rk4(
    initial_state: &OptimalControlState,
    mu: f64,
    thrust: f64,
    effective_exhaust_velocity: f64,
    problem_type: OptimalControlProblemType,
    duration: Duration,
    time_step: Duration,
) -> OptimalControlSolution {
    let total_time = duration.value();
    let dt = time_step.value().clamp(1.0, 3600.0);
    let steps = ((total_time / dt).ceil() as usize).max(2);
    let h = total_time / (steps as f64);

    let mut states = Vec::with_capacity(steps + 1);
    let mut times = Vec::with_capacity(steps + 1);
    let mut h_history = Vec::with_capacity(steps + 1);

    let mut curr = *initial_state;
    let mut t = 0.0;
    let mut acc_dv = 0.0;
    let m0 = initial_state.mass;

    states.push(curr);
    times.push(Duration::new(t));
    h_history.push(compute_hamiltonian(
        &curr,
        mu,
        thrust,
        effective_exhaust_velocity,
        problem_type,
    ));

    for _ in 0..steps {
        if curr.mass <= 1.0 {
            break;
        }

        let k1 = evaluate_optimal_control_dynamics(
            &curr,
            mu,
            thrust,
            effective_exhaust_velocity,
            problem_type,
        );

        let half_h = 0.5 * h;
        let s2 = OptimalControlState {
            position: curr.position + k1.d_position * half_h,
            velocity: curr.velocity + k1.d_velocity * half_h,
            mass: curr.mass + k1.d_mass * half_h,
            lambda_r: curr.lambda_r + k1.d_lambda_r * half_h,
            lambda_v: curr.lambda_v + k1.d_lambda_v * half_h,
            lambda_m: curr.lambda_m + k1.d_lambda_m * half_h,
        };

        let k2 = evaluate_optimal_control_dynamics(
            &s2,
            mu,
            thrust,
            effective_exhaust_velocity,
            problem_type,
        );

        let s3 = OptimalControlState {
            position: curr.position + k2.d_position * half_h,
            velocity: curr.velocity + k2.d_velocity * half_h,
            mass: curr.mass + k2.d_mass * half_h,
            lambda_r: curr.lambda_r + k2.d_lambda_r * half_h,
            lambda_v: curr.lambda_v + k2.d_lambda_v * half_h,
            lambda_m: curr.lambda_m + k2.d_lambda_m * half_h,
        };

        let k3 = evaluate_optimal_control_dynamics(
            &s3,
            mu,
            thrust,
            effective_exhaust_velocity,
            problem_type,
        );

        let s4 = OptimalControlState {
            position: curr.position + k3.d_position * h,
            velocity: curr.velocity + k3.d_velocity * h,
            mass: curr.mass + k3.d_mass * h,
            lambda_r: curr.lambda_r + k3.d_lambda_r * h,
            lambda_v: curr.lambda_v + k3.d_lambda_v * h,
            lambda_m: curr.lambda_m + k3.d_lambda_m * h,
        };

        let k4 = evaluate_optimal_control_dynamics(
            &s4,
            mu,
            thrust,
            effective_exhaust_velocity,
            problem_type,
        );

        let sixth_h = h / 6.0;
        curr.position = curr.position
            + (k1.d_position + k2.d_position * 2.0 + k3.d_position * 2.0 + k4.d_position)
                * sixth_h;
        curr.velocity = curr.velocity
            + (k1.d_velocity + k2.d_velocity * 2.0 + k3.d_velocity * 2.0 + k4.d_velocity)
                * sixth_h;
        curr.mass += (k1.d_mass + k2.d_mass * 2.0 + k3.d_mass * 2.0 + k4.d_mass) * sixth_h;
        curr.lambda_r = curr.lambda_r
            + (k1.d_lambda_r + k2.d_lambda_r * 2.0 + k3.d_lambda_r * 2.0 + k4.d_lambda_r)
                * sixth_h;
        curr.lambda_v = curr.lambda_v
            + (k1.d_lambda_v + k2.d_lambda_v * 2.0 + k3.d_lambda_v * 2.0 + k4.d_lambda_v)
                * sixth_h;
        curr.lambda_m +=
            (k1.d_lambda_m + k2.d_lambda_m * 2.0 + k3.d_lambda_m * 2.0 + k4.d_lambda_m)
                * sixth_h;

        let sw = switching_function(
            curr.lambda_v,
            curr.mass,
            curr.lambda_m,
            effective_exhaust_velocity,
        );
        let u = throttle_command(sw, problem_type);
        acc_dv += (thrust * u / curr.mass) * h;
        t += h;

        states.push(curr);
        times.push(Duration::new(t));
        h_history.push(compute_hamiltonian(
            &curr,
            mu,
            thrust,
            effective_exhaust_velocity,
            problem_type,
        ));
    }

    let prop_used = (m0 - curr.mass).max(0.0);

    OptimalControlSolution {
        states,
        times,
        flight_time: Duration::new(t),
        final_mass: Mass::new(curr.mass),
        total_delta_v: Speed::new(acc_dv),
        propellant_consumed: Mass::new(prop_used),
        hamiltonian_history: h_history,
    }
}