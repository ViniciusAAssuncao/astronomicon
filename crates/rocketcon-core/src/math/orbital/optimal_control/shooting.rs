use super::integration::integrate_optimal_control_rk4;
use super::types::{OptimalControlSolution, OptimalControlState, ShootingBoundaryConditions};
use crate::error::RocketDomainResult;
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{Duration, Vector3};

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
            (
                Vector3::new(p_init.0 * 1e-4, p_init.1 * 1e-4, p_init.2 * 1e-4),
                v_init * 1.0,
                0.0,
            )
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