mod critical_ellipse;
mod dd;

use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::{
    Duration, GravitationalParameter, Position, Vector3, VelocityVector,
};

pub fn stumpff_c(z: f64) -> f64 {
    if !z.is_finite() {
        return f64::NAN;
    }
    if z.abs() <= 0.1 {
        0.5 + z
            * (-1.0 / 24.0
                + z * (1.0 / 720.0
                    + z * (-1.0 / 40320.0 + z * (1.0 / 3628800.0 - z / 479001600.0))))
    } else if z > 0.0 {
        let half = 0.5 * z.sqrt();
        2.0 * half.sin().powi(2) / z
    } else {
        let half = 0.5 * (-z).sqrt();
        2.0 * half.sinh().powi(2) / (-z)
    }
}

pub fn stumpff_s(z: f64) -> f64 {
    if !z.is_finite() {
        return f64::NAN;
    }
    if z.abs() <= 0.1 {
        1.0 / 6.0
            + z * (-1.0 / 120.0
                + z * (1.0 / 5040.0
                    + z * (-1.0 / 362880.0 + z * (1.0 / 39916800.0 - z / 6227020800.0))))
    } else if z > 0.0 {
        let root = z.sqrt();
        (root - root.sin()) / (z * root)
    } else {
        let root = (-z).sqrt();
        (root.sinh() - root) / ((-z) * root)
    }
}

pub fn solve_universal_kepler(
    initial_position: Vector3,
    initial_velocity: Vector3,
    mu: f64,
    dt: f64,
) -> RocketDomainResult<f64> {
    if let Some((_, _, chi)) = critical_ellipse::try_propagate(
        [initial_position.0, initial_position.1, initial_position.2],
        [initial_velocity.0, initial_velocity.1, initial_velocity.2],
        mu,
        dt,
    ) {
        return Ok(chi);
    }
    solve_universal_kepler_f64(initial_position, initial_velocity, mu, dt)
}

fn solve_universal_kepler_f64(
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
    if r0 <= 0.0 || !r0.is_finite() || !v0_sq.is_finite() || !dt.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "initial_state".to_string(),
            reason: "initial state and elapsed time must be finite with positive radius"
                .to_string(),
        });
    }
    if dt == 0.0 {
        return Ok(0.0);
    }
    let mu_sqrt = mu.sqrt();
    let sigma0 = initial_position.dot(&initial_velocity) / mu_sqrt;
    let alpha = 2.0 / r0 - v0_sq / mu;
    if !alpha.is_finite() || !sigma0.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "initial_state".to_string(),
            reason: "universal elements are not finite".to_string(),
        });
    }
    let sign = dt.signum();
    let target = mu_sqrt * dt;
    if !target.is_finite() {
        return Err(RocketDomainError::NumericalConvergence {
            context: "universal_kepler_solver".to_string(),
            reason: "elapsed time exceeds representable universal anomaly range".to_string(),
        });
    }

    let eval = |x: f64| {
        let chi = sign * x;
        let chi2 = chi * chi;
        let z = alpha * chi2;
        let c = stumpff_c(z);
        let s = stumpff_s(z);
        let f = sigma0 * chi2 * c + (1.0 - alpha * r0) * chi * chi2 * s + r0 * chi - target;
        let derivative = sigma0 * chi * (1.0 - z * s) + (1.0 - alpha * r0) * chi2 * c + r0;
        (sign * f, derivative)
    };

    let normalized_alpha = alpha * r0;
    let baseline = mu_sqrt * dt.abs() / r0;
    let seed = if normalized_alpha > 1e-6 {
        mu_sqrt * dt.abs() * alpha
    } else if alpha < 0.0 {
        let sqrt_minus_alpha = (-alpha).sqrt();
        let numerator = 2.0 * mu * (-alpha) * dt.abs();
        let denominator = sign * initial_position.dot(&initial_velocity)
            + (mu / (-alpha)).sqrt() * (1.0 - alpha * r0);
        if denominator > 0.0 && (numerator / denominator) > 1.0 {
            (numerator / denominator).ln() / sqrt_minus_alpha
        } else {
            (baseline * sqrt_minus_alpha).ln_1p() / sqrt_minus_alpha
        }
    } else {
        baseline
    };
    if seed.is_finite() && seed > 0.0 {
        let mut candidate = seed;
        for _ in 0..8 {
            let (residual, derivative) = eval(candidate);
            if !residual.is_finite() || !derivative.is_finite() || derivative <= 0.0 {
                break;
            }
            let scale = target.abs().max(r0 * candidate).max(f64::MIN_POSITIVE);
            let correction = residual / derivative;
            let condition = derivative * candidate / scale;
            if residual.abs() <= 2e-13 * scale
                && (correction.abs() <= 2e-11 * candidate || condition >= 0.01)
            {
                return Ok(sign * candidate);
            }
            let next = candidate - correction;
            if !next.is_finite() || next <= 0.0 || correction.abs() > 2.0 * candidate {
                break;
            }
            if next == candidate {
                break;
            }
            candidate = next;
        }
    }

    let mut hi = if seed.is_finite() && seed > 0.0 {
        seed
    } else {
        baseline
    };
    if !hi.is_finite() || hi <= 0.0 {
        return Err(RocketDomainError::NumericalConvergence {
            context: "universal_kepler_solver".to_string(),
            reason: "failed to construct initial universal anomaly".to_string(),
        });
    }

    let mut bracketed = false;
    for _ in 0..96 {
        let (f, _) = eval(hi);
        if f.is_nan() {
            break;
        }
        if f >= 0.0 {
            bracketed = true;
            break;
        }
        hi *= 2.0;
        if !hi.is_finite() {
            break;
        }
    }
    if !bracketed {
        return Err(RocketDomainError::NumericalConvergence {
            context: "universal_kepler_solver".to_string(),
            reason: "could not bracket the universal Kepler root".to_string(),
        });
    }

    let mut lo = 0.0;
    let mut x = (0.5 * hi).max(f64::MIN_POSITIVE);
    for _ in 0..128 {
        let (f, derivative) = eval(x);
        if !f.is_finite() {
            return Err(RocketDomainError::NumericalConvergence {
                context: "universal_kepler_solver".to_string(),
                reason: "non-finite universal Kepler residual".to_string(),
            });
        }
        if f >= 0.0 {
            hi = x;
        } else {
            lo = x;
        }
        let residual_scale = target.abs().max(r0 * x).max(f64::MIN_POSITIVE);
        let correction = if derivative.is_finite() && derivative > 0.0 {
            f / derivative
        } else {
            f64::INFINITY
        };
        let condition = derivative * x / residual_scale;
        if f.abs() <= 2e-13 * residual_scale && (correction.abs() <= 2e-11 * x || condition >= 0.01)
        {
            return Ok(sign * x);
        }
        if hi - lo <= 8.0 * f64::EPSILON * hi.max(f64::MIN_POSITIVE) {
            return Err(RocketDomainError::NumericalConvergence {
                context: "universal_kepler_solver".to_string(),
                reason: format!(
                    "root stagnated: relative residual={:.3e}, root conditioning={:.3e}, relative estimated correction={:.3e}",
                    f.abs() / residual_scale,
                    condition,
                    correction.abs() / x
                ),
            });
        }
        let candidate = x - correction;
        if !derivative.is_finite()
            || derivative <= 0.0
            || !candidate.is_finite()
            || candidate <= lo + 0.01 * (hi - lo)
            || candidate >= hi - 0.01 * (hi - lo)
        {
            x = lo + 0.5 * (hi - lo);
        } else {
            x = candidate;
        }
    }
    Err(RocketDomainError::NumericalConvergence {
        context: "universal_kepler_solver".to_string(),
        reason: "bracketed root search exhausted its iteration budget".to_string(),
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

    if let Some((p, v, _)) = critical_ellipse::try_propagate(
        [r_vec.0, r_vec.1, r_vec.2],
        [v_vec.0, v_vec.1, v_vec.2],
        mu_val,
        dt_val,
    ) {
        return Ok((
            Position::from_raw(Vector3::new(p[0], p[1], p[2])),
            VelocityVector::from_raw(Vector3::new(v[0], v[1], v[2])),
        ));
    }

    let chi = solve_universal_kepler_f64(r_vec, v_vec, mu_val, dt_val)?;
    if dt_val == 0.0 {
        return Ok((initial_position, initial_velocity));
    }
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
    if ![
        r_final.0, r_final.1, r_final.2, v_final.0, v_final.1, v_final.2,
    ]
    .iter()
    .all(|coordinate| coordinate.is_finite())
    {
        return Err(RocketDomainError::NumericalConvergence {
            context: "universal_state_propagation".to_string(),
            reason: "propagated state is not finite".to_string(),
        });
    }

    Ok((
        Position::from_raw(r_final),
        VelocityVector::from_raw(v_final),
    ))
}
