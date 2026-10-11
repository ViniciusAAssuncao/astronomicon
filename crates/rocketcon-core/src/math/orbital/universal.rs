use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::{Duration, GravitationalParameter, Position, Vector3, VelocityVector};

pub fn stumpff_c(z: f64) -> f64 {
    if !z.is_finite() {
        return 0.5;
    }
    if z > 1e-6 {
        let sqrt_z = z.sqrt();
        (1.0 - sqrt_z.cos()) / z
    } else if z < -1e-6 {
        let sqrt_neg_z = (-z).sqrt();
        (sqrt_neg_z.cosh() - 1.0) / (-z)
    } else {
        0.5 - z / 24.0 + (z * z) / 720.0 - (z * z * z) / 40320.0
    }
}

pub fn stumpff_s(z: f64) -> f64 {
    if !z.is_finite() {
        return 1.0 / 6.0;
    }
    if z > 1e-6 {
        let sqrt_z = z.sqrt();
        (sqrt_z - sqrt_z.sin()) / (z * sqrt_z)
    } else if z < -1e-6 {
        let sqrt_neg_z = (-z).sqrt();
        (sqrt_neg_z.sinh() - sqrt_neg_z) / ((-z) * sqrt_neg_z)
    } else {
        1.0 / 6.0 - z / 120.0 + (z * z) / 5040.0 - (z * z * z) / 362880.0
    }
}

pub fn solve_universal_kepler(
    initial_position: Vector3,
    initial_velocity: Vector3,
    mu: f64,
    dt: f64,
) -> RocketDomainResult<f64> {
    if mu <= 0.0 || !mu.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "mu".to_string(),
            reason: "gravitational parameter must be positive and finite".to_string(),
        });
    }

    let r0 = initial_position.magnitude();
    let v0_sq = initial_velocity.dot(&initial_velocity);

    if r0 <= 1e-6 || !r0.is_finite() || !v0_sq.is_finite() || !dt.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "initial_state".to_string(),
            reason: "initial position and velocity must be non-zero and finite".to_string(),
        });
    }

    if dt == 0.0 {
        return Ok(0.0);
    }

    let mu_sqrt = mu.sqrt();
    let r0_dot_v0 = initial_position.dot(&initial_velocity);
    let sigma0 = r0_dot_v0 / mu_sqrt;
    let alpha = 2.0 / r0 - v0_sq / mu;

    let mut chi = if alpha > 1e-6 {
        mu_sqrt * dt * alpha
    } else if alpha < -1e-6 {
        let term = -2.0 * mu * alpha * dt;
        let denom = sigma0 * mu_sqrt + dt.signum() * (-mu / alpha).sqrt() * (1.0 - alpha * r0);
        if denom.abs() > 1e-12 && term / denom > 0.0 {
            dt.signum() * (1.0 / (-alpha).sqrt()) * (term / denom).ln()
        } else {
            mu_sqrt * dt / r0
        }
    } else {
        mu_sqrt * dt / r0
    };

    let max_iter = 100;
    let tol = 1e-12;

    for _ in 0..max_iter {
        let chi2 = chi * chi;
        let z = alpha * chi2;
        let c = stumpff_c(z);
        let s = stumpff_s(z);

        let f = sigma0 * chi2 * c + (1.0 - alpha * r0) * chi * chi2 * s + r0 * chi - mu_sqrt * dt;
        let f_prime = sigma0 * chi * (1.0 - z * s) + (1.0 - alpha * r0) * chi2 * c + r0;

        if f_prime.abs() < 1e-15 || !f_prime.is_finite() {
            break;
        }

        let delta = f / f_prime;
        chi -= delta;

        if delta.abs() < tol {
            return Ok(chi);
        }
    }

    Err(RocketDomainError::NumericalConvergence {
        context: "universal_kepler_solver".to_string(),
        reason: "failed to converge within maximum iterations".to_string(),
    })
}

pub fn propagate_universal_state_vectors(
    initial_position: Position,
    initial_velocity: VelocityVector,
    mu: GravitationalParameter,
    dt: Duration,
) -> RocketDomainResult<(Position, VelocityVector)> {
    let r_vec = initial_position.raw();
    let v_vec = initial_velocity.raw();
    let mu_val = mu.value();
    let dt_val = dt.value();

    if dt_val == 0.0 {
        return Ok((initial_position, initial_velocity));
    }

    let chi = solve_universal_kepler(r_vec, v_vec, mu_val, dt_val)?;
    let r0 = r_vec.magnitude();
    let mu_sqrt = mu_val.sqrt();
    let r0_dot_v0 = r_vec.dot(&v_vec);
    let sigma0 = r0_dot_v0 / mu_sqrt;
    let v0_sq = v_vec.dot(&v_vec);
    let alpha = 2.0 / r0 - v0_sq / mu_val;

    let chi2 = chi * chi;
    let z = alpha * chi2;
    let c = stumpff_c(z);
    let s = stumpff_s(z);

    let f_coeff = 1.0 - (chi2 / r0) * c;
    let g_coeff = dt_val - (chi2 * chi / mu_sqrt) * s;

    let r_mag = sigma0 * chi * (1.0 - z * s) + (1.0 - alpha * r0) * chi2 * c + r0;
    if r_mag <= 0.0 || !r_mag.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "propagated_radius".to_string(),
            reason: "propagated radial distance is non-positive or non-finite".to_string(),
        });
    }

    let f_dot = (mu_sqrt / (r_mag * r0)) * chi * (z * s - 1.0);
    let g_dot = 1.0 - (chi2 / r_mag) * c;

    let r_final = r_vec * f_coeff + v_vec * g_coeff;
    let v_final = r_vec * f_dot + v_vec * g_dot;

    Ok((
        Position::from_raw(r_final),
        VelocityVector::from_raw(v_final),
    ))
}