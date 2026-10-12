use super::jacobi::compute_jacobi_constant;
use super::richardson::richardson_halo_approximation;
use super::types::{Cr3bpParameters, HaloOrbitFamily, HaloOrbitState, LibrationPoint, SynodicState};
use super::variational::{variational_rk4_step, Variational42State};
use crate::constants::{CR3BP_HALO_CORRECTION_MAX_ITERATIONS, CR3BP_HALO_CORRECTION_TOLERANCE};
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::Vector3;

pub fn refine_halo_orbit(
    params: &Cr3bpParameters,
    point: LibrationPoint,
    family: HaloOrbitFamily,
    az_amplitude_dimensionless: f64,
) -> RocketDomainResult<HaloOrbitState> {
    let (initial_guess, period_guess) =
        richardson_halo_approximation(params, point, family, az_amplitude_dimensionless);

    let mut x0 = initial_guess.position.0;
    let z0 = initial_guess.position.2;
    let mut vy0 = initial_guess.velocity.1;
    let mu = params.mu;

    let mut final_half_period = 0.5 * period_guess;
    let mut converged = false;

    let dt = 1e-3;

    for _ in 0..CR3BP_HALO_CORRECTION_MAX_ITERATIONS {
        let current_state = SynodicState::new(
            Vector3::new(x0, 0.0, z0),
            Vector3::new(0.0, vy0, 0.0),
        );

        let mut var_state = Variational42State::new(current_state);
        let mut t = 0.0;
        let mut prev_var = var_state;
        let mut prev_t = 0.0;

        let max_t = period_guess * 1.5;
        while t < max_t {
            prev_var = var_state;
            prev_t = t;
            var_state = variational_rk4_step(&var_state, mu, dt);
            t += dt;

            if t > 0.05 && prev_var.state.position.1 * var_state.state.position.1 <= 0.0 {
                break;
            }
        }

        let y1 = prev_var.state.position.1;
        let y2 = var_state.state.position.1;
        let frac = if (y2 - y1).abs() > 1e-15 { -y1 / (y2 - y1) } else { 0.0 };
        let t_half = prev_t + frac * dt;
        final_half_period = t_half;

        let half_step = t_half - prev_t;
        let cross_var = variational_rk4_step(&prev_var, mu, half_step);

        let vx = cross_var.state.velocity.0;
        let vz = cross_var.state.velocity.2;
        let vy = cross_var.state.velocity.1;

        let error_norm = (vx * vx + vz * vz).sqrt();
        if error_norm < CR3BP_HALO_CORRECTION_TOLERANCE {
            converged = true;
            break;
        }

        let stm = cross_var.stm_matrix();
        let acc = super::equations::cr3bp_acceleration(cross_var.state.position, cross_var.state.velocity, mu);

        let ax = acc.0;
        let az = acc.2;

        let j11 = stm[3][0] - (ax / vy.max(1e-12)) * stm[1][0];
        let j12 = stm[3][4] - (ax / vy.max(1e-12)) * stm[1][4];
        let j21 = stm[5][0] - (az / vy.max(1e-12)) * stm[1][0];
        let j22 = stm[5][4] - (az / vy.max(1e-12)) * stm[1][4];

        let det = j11 * j22 - j12 * j21;
        if det.abs() < 1e-15 {
            break;
        }

        let delta_x0 = -(j22 * vx - j12 * vz) / det;
        let delta_vy0 = -(-j21 * vx + j11 * vz) / det;

        let damping = 0.8;
        x0 += delta_x0 * damping;
        vy0 += delta_vy0 * damping;
    }

    let final_init = SynodicState::new(
        Vector3::new(x0, 0.0, z0),
        Vector3::new(0.0, vy0, 0.0),
    );

    let period = final_half_period * 2.0;
    let jacobi = compute_jacobi_constant(final_init.position, final_init.velocity, mu);

    if !converged {
        return Err(RocketDomainError::NumericalConvergence {
            context: "halo_orbit_differential_corrector".to_string(),
            reason: "failed to converge to requested tolerance".to_string(),
        });
    }

    Ok(HaloOrbitState {
        initial_state: final_init,
        period_dimensionless: period,
        jacobi_constant: jacobi,
        libration_point: point,
        family,
        az_amplitude_dimensionless,
    })
}