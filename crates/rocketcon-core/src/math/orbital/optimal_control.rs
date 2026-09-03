use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::low_thrust::ChebyshevTrajectoryApproximation;
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{
    Duration, Force, GravitationalParameter, Mass, Position, Speed, Vector3, VelocityVector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OptimalControlProblemType {
    MinimumFuel,
    MinimumTime,
    EnergyOptimal { smoothing_epsilon: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OptimalControlState {
    pub position: Vector3,
    pub velocity: Vector3,
    pub mass: f64,
    pub lambda_r: Vector3,
    pub lambda_v: Vector3,
    pub lambda_m: f64,
}

impl OptimalControlState {
    pub fn new(
        position: Vector3,
        velocity: Vector3,
        mass: f64,
        lambda_r: Vector3,
        lambda_v: Vector3,
        lambda_m: f64,
    ) -> Self {
        Self {
            position,
            velocity,
            mass,
            lambda_r,
            lambda_v,
            lambda_m,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OptimalControlDerivative {
    pub d_position: Vector3,
    pub d_velocity: Vector3,
    pub d_mass: f64,
    pub d_lambda_r: Vector3,
    pub d_lambda_v: Vector3,
    pub d_lambda_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimalControlSolution {
    pub states: Vec<OptimalControlState>,
    pub times: Vec<Duration>,
    pub flight_time: Duration,
    pub final_mass: Mass,
    pub total_delta_v: Speed,
    pub propellant_consumed: Mass,
    pub hamiltonian_history: Vec<f64>,
}

impl OptimalControlSolution {
    pub fn to_chebyshev(
        &self,
        start_epoch: Duration,
        degree: usize,
    ) -> RocketDomainResult<ChebyshevTrajectoryApproximation> {
        let n = self.states.len();
        if n < degree + 1 {
            return Err(RocketDomainError::InvalidInvariant {
                field: "states".to_string(),
                reason: "insufficient states to fit chebyshev polynomial".to_string(),
            });
        }

        let tf = self.flight_time.value();
        let mut tau_samples = Vec::with_capacity(n);
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        let mut zs = Vec::with_capacity(n);
        let mut vxs = Vec::with_capacity(n);
        let mut vys = Vec::with_capacity(n);
        let mut vzs = Vec::with_capacity(n);
        let mut masses = Vec::with_capacity(n);

        for (state, time) in self.states.iter().zip(self.times.iter()) {
            let t = time.value();
            let tau = if tf > 0.0 { (2.0 * t / tf) - 1.0 } else { 0.0 };
            tau_samples.push(tau.clamp(-1.0, 1.0));
            xs.push(state.position.0);
            ys.push(state.position.1);
            zs.push(state.position.2);
            vxs.push(state.velocity.0);
            vys.push(state.velocity.1);
            vzs.push(state.velocity.2);
            masses.push(state.mass);
        }

        let c_x = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &xs,
            degree,
        )?;
        let c_y = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &ys,
            degree,
        )?;
        let c_z = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &zs,
            degree,
        )?;
        let c_vx = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &vxs,
            degree,
        )?;
        let c_vy = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &vys,
            degree,
        )?;
        let c_vz = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &vzs,
            degree,
        )?;
        let c_mass = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &masses,
            degree,
        )?;

        Ok(ChebyshevTrajectoryApproximation {
            start_epoch,
            end_epoch: start_epoch + self.flight_time,
            degree,
            c_x,
            c_y,
            c_z,
            c_vx,
            c_vy,
            c_vz,
            c_mass,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShootingBoundaryConditions {
    pub initial_position: Position,
    pub initial_velocity: VelocityVector,
    pub initial_mass: Mass,
    pub target_position: Position,
    pub target_velocity: VelocityVector,
    pub time_of_flight: Duration,
    pub thrust: Force,
    pub specific_impulse: Duration,
    pub mu: GravitationalParameter,
    pub problem_type: OptimalControlProblemType,
}

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
    effective_exhaust_velocity: f64,
) -> f64 {
    if mass <= 0.0 || effective_exhaust_velocity <= 0.0 {
        return 0.0;
    }
    (lambda_v.magnitude() / mass) - (lambda_m / effective_exhaust_velocity)
}

pub fn throttle_command(
    switching_val: f64,
    problem_type: OptimalControlProblemType,
) -> f64 {
    match problem_type {
        OptimalControlProblemType::MinimumTime => 1.0,
        OptimalControlProblemType::MinimumFuel => {
            if switching_val > 0.0 {
                1.0
            } else {
                0.0
            }
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
    problem_type: OptimalControlProblemType,
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
        effective_exhaust_velocity,
    );
    let u = throttle_command(sw, problem_type);
    let u_thrust = primer_vector_optimal_thrust_direction(state.lambda_v);
    let a_thrust = u_thrust * (thrust * u / state.mass);

    let h_kin = state.lambda_r.dot(&state.velocity);
    let h_acc = state.lambda_v.dot(&(a_grav + a_thrust));
    let h_mass = -state.lambda_m * (thrust * u / effective_exhaust_velocity);

    let l_term = match problem_type {
        OptimalControlProblemType::MinimumTime => 1.0,
        OptimalControlProblemType::MinimumFuel => thrust * u / effective_exhaust_velocity,
        OptimalControlProblemType::EnergyOptimal { .. } => 0.5 * u * u,
    };

    h_kin + h_acc + h_mass + l_term
}

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
        curr.position =
            curr.position + (k1.d_position + k2.d_position * 2.0 + k3.d_position * 2.0 + k4.d_position) * sixth_h;
        curr.velocity =
            curr.velocity + (k1.d_velocity + k2.d_velocity * 2.0 + k3.d_velocity * 2.0 + k4.d_velocity) * sixth_h;
        curr.mass += (k1.d_mass + k2.d_mass * 2.0 + k3.d_mass * 2.0 + k4.d_mass) * sixth_h;
        curr.lambda_r =
            curr.lambda_r + (k1.d_lambda_r + k2.d_lambda_r * 2.0 + k3.d_lambda_r * 2.0 + k4.d_lambda_r) * sixth_h;
        curr.lambda_v =
            curr.lambda_v + (k1.d_lambda_v + k2.d_lambda_v * 2.0 + k3.d_lambda_v * 2.0 + k4.d_lambda_v) * sixth_h;
        curr.lambda_m += (k1.d_lambda_m + k2.d_lambda_m * 2.0 + k3.d_lambda_m * 2.0 + k4.d_lambda_m) * sixth_h;

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

fn solve_linear_system_6x6(mut a: [[f64; 6]; 6], mut b: [f64; 6]) -> Option<[f64; 6]> {
    for i in 0..6 {
        let mut pivot_row = i;
        let mut max_val = a[i][i].abs();
        for k in (i + 1)..6 {
            if a[k][i].abs() > max_val {
                max_val = a[k][i].abs();
                pivot_row = k;
            }
        }
        if max_val < 1e-15 {
            return None;
        }
        if pivot_row != i {
            a.swap(i, pivot_row);
            b.swap(i, pivot_row);
        }
        let diag = a[i][i];
        for j in i..6 {
            a[i][j] /= diag;
        }
        b[i] /= diag;
        for k in 0..6 {
            if k != i {
                let factor = a[k][i];
                for j in i..6 {
                    a[k][j] -= factor * a[i][j];
                }
                b[k] -= factor * b[i];
            }
        }
    }
    Some(b)
}

pub fn solve_optimal_control_shooting(
    conditions: &ShootingBoundaryConditions,
    initial_costate_guess: Option<(Vector3, Vector3, f64)>,
    tolerance: f64,
    max_iterations: usize,
) -> RocketDomainResult<OptimalControlSolution> {
    let mu = conditions.mu.value();
    let thrust = conditions.thrust.value();
    let isp = conditions.specific_impulse.value();
    let ve = isp * STANDARD_GRAVITY;
    let r0 = conditions.initial_position.raw();
    let v0 = conditions.initial_velocity.raw();
    let rf = conditions.target_position.raw();
    let vf = conditions.target_velocity.raw();
    let m0 = conditions.initial_mass.value();
    let tf = conditions.time_of_flight;

    let dt = Duration::new((tf.value() / 200.0).clamp(1.0, 600.0));

    let (mut lr, mut lv, mut lm) = match initial_costate_guess {
        Some((g_lr, g_lv, g_lm)) => (g_lr, g_lv, g_lm),
        None => {
            let dr = rf - r0;
            let dv = vf - v0;
            let p_init = dr.normalized();
            let v_init = dv.normalized();
            (Vector3::new(p_init.0 * 1e-4, p_init.1 * 1e-4, p_init.2 * 1e-4), v_init * 1.0, 0.0)
        }
    };

    let tol = tolerance.max(1.0);

    for _ in 0..max_iterations {
        let init_state = OptimalControlState::new(r0, v0, m0, lr, lv, lm);
        let sol = integrate_optimal_control_rk4(
            &init_state,
            mu,
            thrust,
            ve,
            conditions.problem_type,
            tf,
            dt,
        );

        let final_state = sol.states.last().copied().unwrap_or(init_state);
        let err_r = final_state.position - rf;
        let err_v = final_state.velocity - vf;

        let res = [
            err_r.0, err_r.1, err_r.2,
            err_v.0, err_v.1, err_v.2,
        ];

        let max_err = err_r.magnitude().max(err_v.magnitude() * 1000.0);
        if max_err <= tol {
            return Ok(sol);
        }

        let mut jac = [[0.0; 6]; 6];
        let p_vars = [
            lr.0, lr.1, lr.2,
            lv.0, lv.1, lv.2,
        ];

        for col in 0..6 {
            let mut perturbed_vars = p_vars;
            let eps = 1e-7 * p_vars[col].abs().max(1.0);
            perturbed_vars[col] += eps;

            let pert_lr = Vector3::new(perturbed_vars[0], perturbed_vars[1], perturbed_vars[2]);
            let pert_lv = Vector3::new(perturbed_vars[3], perturbed_vars[4], perturbed_vars[5]);
            let pert_state = OptimalControlState::new(r0, v0, m0, pert_lr, pert_lv, lm);

            let pert_sol = integrate_optimal_control_rk4(
                &pert_state,
                mu,
                thrust,
                ve,
                conditions.problem_type,
                tf,
                dt,
            );

            let pert_final = pert_sol.states.last().copied().unwrap_or(pert_state);
            let pert_err_r = pert_final.position - rf;
            let pert_err_v = pert_final.velocity - vf;

            jac[0][col] = (pert_err_r.0 - err_r.0) / eps;
            jac[1][col] = (pert_err_r.1 - err_r.1) / eps;
            jac[2][col] = (pert_err_r.2 - err_r.2) / eps;
            jac[3][col] = (pert_err_v.0 - err_v.0) / eps;
            jac[4][col] = (pert_err_v.1 - err_v.1) / eps;
            jac[5][col] = (pert_err_v.2 - err_v.2) / eps;
        }

        let delta_p = match solve_linear_system_6x6(jac, res) {
            Some(dp) => dp,
            None => {
                break;
            }
        };

        let damping = 0.5;
        lr.0 -= damping * delta_p[0];
        lr.1 -= damping * delta_p[1];
        lr.2 -= damping * delta_p[2];
        lv.0 -= damping * delta_p[3];
        lv.1 -= damping * delta_p[4];
        lv.2 -= damping * delta_p[5];
        lm = lm.clamp(-10.0, 10.0);
    }

    let final_init = OptimalControlState::new(r0, v0, m0, lr, lv, lm);
    Ok(integrate_optimal_control_rk4(
        &final_init,
        mu,
        thrust,
        ve,
        conditions.problem_type,
        tf,
        dt,
    ))
}