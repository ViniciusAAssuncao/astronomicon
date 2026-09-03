use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::{
    Angle, AngularVelocity, Duration, GravitationalParameter, Length, Position, Speed, Vector3,
    VelocityVector,
};

pub fn solve_hyperbolic_kepler(mean_anomaly: f64, eccentricity: f64) -> RocketDomainResult<f64> {
    if eccentricity <= 1.0 || !eccentricity.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "eccentricity".to_string(),
            reason: "eccentricity must be strictly greater than 1.0 for hyperbolic trajectory"
                .to_string(),
        });
    }

    if !mean_anomaly.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "mean_anomaly".to_string(),
            reason: "mean anomaly must be finite".to_string(),
        });
    }

    let m = mean_anomaly;
    let e = eccentricity;

    let mut h = if m.abs() < 1.0 {
        m / (e - 1.0)
    } else {
        let sign = m.signum();
        let m_abs = m.abs();
        sign * ((2.0 * m_abs / e).ln() + 0.5)
    };

    let max_iter = 100;
    let tol = 1e-12;

    for _ in 0..max_iter {
        let sinh_h = h.sinh();
        let cosh_h = h.cosh();

        let f = e * sinh_h - h - m;
        let f_prime = e * cosh_h - 1.0;
        let f_double_prime = e * sinh_h;

        let denom = f_prime - (f * f_double_prime) / (2.0 * f_prime);
        if denom.abs() < 1e-15 || !denom.is_finite() {
            let delta = f / f_prime;
            h -= delta;
            if delta.abs() < tol {
                return Ok(h);
            }
        } else {
            let delta = f / denom;
            h -= delta;
            if delta.abs() < tol {
                return Ok(h);
            }
        }
    }

    Err(RocketDomainError::NumericalConvergence {
        context: "hyperbolic_kepler_solver".to_string(),
        reason: "failed to converge within maximum iterations".to_string(),
    })
}

pub fn true_anomaly_from_hyperbolic_anomaly(hyperbolic_anomaly: f64, eccentricity: f64) -> Angle {
    if eccentricity <= 1.0 || !eccentricity.is_finite() || !hyperbolic_anomaly.is_finite() {
        return Angle::new(0.0);
    }
    let factor = ((eccentricity + 1.0) / (eccentricity - 1.0)).sqrt();
    let tanh_half_h = (0.5 * hyperbolic_anomaly).tanh();
    let tan_half_nu = factor * tanh_half_h;
    Angle::new(2.0 * tan_half_nu.atan())
}

pub fn hyperbolic_anomaly_from_true_anomaly(
    true_anomaly: Angle,
    eccentricity: f64,
) -> RocketDomainResult<f64> {
    if eccentricity <= 1.0 || !eccentricity.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "eccentricity".to_string(),
            reason: "eccentricity must be strictly greater than 1.0 for hyperbolic trajectory"
                .to_string(),
        });
    }

    let nu = true_anomaly.value();
    if !nu.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "true_anomaly".to_string(),
            reason: "true anomaly must be finite".to_string(),
        });
    }

    let limit = (-1.0 / eccentricity).clamp(-1.0, 1.0).acos();
    if nu.abs() >= limit - 1e-10 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "true_anomaly".to_string(),
            reason: format!(
                "true anomaly magnitude {} exceeds asymptote limit {}",
                nu.abs(),
                limit
            ),
        });
    }

    let factor = ((eccentricity - 1.0) / (eccentricity + 1.0)).sqrt();
    let tan_half_nu = (0.5 * nu).tan();
    let tanh_half_h = (factor * tan_half_nu).clamp(-0.999999999999, 0.999999999999);
    Ok(2.0 * tanh_half_h.atanh())
}

pub fn hyperbolic_mean_anomaly(hyperbolic_anomaly: f64, eccentricity: f64) -> f64 {
    if !hyperbolic_anomaly.is_finite() || !eccentricity.is_finite() {
        0.0
    } else {
        eccentricity * hyperbolic_anomaly.sinh() - hyperbolic_anomaly
    }
}

pub fn hyperbolic_mean_anomaly_from_true_anomaly(
    true_anomaly: Angle,
    eccentricity: f64,
) -> RocketDomainResult<f64> {
    let h = hyperbolic_anomaly_from_true_anomaly(true_anomaly, eccentricity)?;
    Ok(hyperbolic_mean_anomaly(h, eccentricity))
}

pub fn hyperbolic_mean_motion(
    semi_major_axis: Length,
    mu: GravitationalParameter,
) -> AngularVelocity {
    let a = semi_major_axis.value();
    let mu_val = mu.value();
    if a >= 0.0 || mu_val <= 0.0 || !a.is_finite() || !mu_val.is_finite() {
        AngularVelocity::new(0.0)
    } else {
        AngularVelocity::new((mu_val / (-a).powi(3)).sqrt())
    }
}

pub fn hyperbolic_excess_velocity(
    semi_major_axis: Length,
    mu: GravitationalParameter,
) -> Speed {
    let a = semi_major_axis.value();
    let mu_val = mu.value();
    if a >= 0.0 || mu_val <= 0.0 || !a.is_finite() || !mu_val.is_finite() {
        Speed::new(0.0)
    } else {
        Speed::new((mu_val / (-a)).sqrt())
    }
}

pub fn hyperbolic_true_anomaly_asymptote(eccentricity: f64) -> Angle {
    if eccentricity <= 1.0 || !eccentricity.is_finite() {
        Angle::new(0.0)
    } else {
        Angle::new((-1.0 / eccentricity).clamp(-1.0, 1.0).acos())
    }
}

pub fn hyperbolic_turn_angle(eccentricity: f64) -> Angle {
    if eccentricity <= 1.0 || !eccentricity.is_finite() {
        Angle::new(0.0)
    } else {
        Angle::new(2.0 * (1.0 / eccentricity).clamp(-1.0, 1.0).asin())
    }
}

pub fn hyperbolic_impact_parameter(semi_major_axis: Length, eccentricity: f64) -> Length {
    let a = semi_major_axis.value();
    let e = eccentricity;
    if a >= 0.0 || e <= 1.0 || !a.is_finite() || !e.is_finite() {
        Length::new(0.0)
    } else {
        Length::new((-a) * (e * e - 1.0).sqrt())
    }
}

pub fn hyperbolic_radius(
    semi_major_axis: Length,
    eccentricity: f64,
    hyperbolic_anomaly: f64,
) -> Length {
    let a = semi_major_axis.value();
    let e = eccentricity;
    let h = hyperbolic_anomaly;
    if a >= 0.0 || e <= 1.0 || !a.is_finite() || !e.is_finite() || !h.is_finite() {
        Length::new(0.0)
    } else {
        Length::new((-a) * (e * h.cosh() - 1.0))
    }
}

pub fn hyperbolic_radius_from_true_anomaly(
    semi_major_axis: Length,
    eccentricity: f64,
    true_anomaly: Angle,
) -> Length {
    let a = semi_major_axis.value();
    let e = eccentricity;
    let nu = true_anomaly.value();

    if a >= 0.0 || e <= 1.0 || !a.is_finite() || !e.is_finite() || !nu.is_finite() {
        return Length::new(0.0);
    }

    let p = (-a) * (e * e - 1.0);
    let denom = 1.0 + e * nu.cos();
    if denom <= 1e-12 {
        Length::new(f64::INFINITY)
    } else {
        Length::new(p / denom)
    }
}

pub fn hyperbolic_orbital_speed(
    mu: GravitationalParameter,
    radius: Length,
    semi_major_axis: Length,
) -> Speed {
    let mu_val = mu.value();
    let r = radius.value();
    let a = semi_major_axis.value();

    if mu_val <= 0.0 || r <= 0.0 || a >= 0.0 || !mu_val.is_finite() || !r.is_finite() || !a.is_finite()
    {
        return Speed::new(0.0);
    }

    let v_sq = mu_val * (2.0 / r - 1.0 / a);
    if v_sq <= 0.0 || !v_sq.is_finite() {
        Speed::new(0.0)
    } else {
        Speed::new(v_sq.sqrt())
    }
}

pub fn hyperbolic_flight_path_angle(eccentricity: f64, true_anomaly: Angle) -> Angle {
    let e = eccentricity;
    let nu = true_anomaly.value();
    if e <= 1.0 || !e.is_finite() || !nu.is_finite() {
        Angle::new(0.0)
    } else {
        let sin_part = e * nu.sin();
        let cos_part = 1.0 + e * nu.cos();
        Angle::new(sin_part.atan2(cos_part))
    }
}

pub fn propagate_hyperbolic_anomaly(
    initial_h: f64,
    eccentricity: f64,
    semi_major_axis: Length,
    mu: GravitationalParameter,
    delta_t: Duration,
) -> RocketDomainResult<f64> {
    let m_h0 = hyperbolic_mean_anomaly(initial_h, eccentricity);
    let n_h = hyperbolic_mean_motion(semi_major_axis, mu).value();
    let m_h_target = m_h0 + n_h * delta_t.value();
    solve_hyperbolic_kepler(m_h_target, eccentricity)
}

pub fn hyperbolic_state_vectors(
    semi_major_axis: Length,
    eccentricity: f64,
    true_anomaly: Angle,
    inclination: Angle,
    longitude_of_ascending_node: Angle,
    argument_of_periapsis: Angle,
    mu: GravitationalParameter,
) -> RocketDomainResult<(Position, VelocityVector)> {
    let a = semi_major_axis.value();
    let e = eccentricity;
    let nu = true_anomaly.value();
    let inc = inclination.value();
    let raan = longitude_of_ascending_node.value();
    let omega = argument_of_periapsis.value();
    let mu_val = mu.value();

    if a >= 0.0 || e <= 1.0 || mu_val <= 0.0 || !a.is_finite() || !e.is_finite() || !mu_val.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "hyperbolic_elements".to_string(),
            reason: "invalid parameters for hyperbolic state vectors".to_string(),
        });
    }

    let p = (-a) * (e * e - 1.0);
    let denom = 1.0 + e * nu.cos();
    if denom <= 1e-12 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "true_anomaly".to_string(),
            reason: "true anomaly is at or beyond hyperbolic asymptote".to_string(),
        });
    }

    let r = p / denom;
    let r_pqw = Vector3::new(r * nu.cos(), r * nu.sin(), 0.0);

    let h = (mu_val * p).sqrt();
    let v_factor = mu_val / h;
    let v_pqw = Vector3::new(-v_factor * nu.sin(), v_factor * (e + nu.cos()), 0.0);

    let r_inertial = r_pqw
        .rotate_about_z(omega)
        .rotate_about_x(inc)
        .rotate_about_z(raan);

    let v_inertial = v_pqw
        .rotate_about_z(omega)
        .rotate_about_x(inc)
        .rotate_about_z(raan);

    Ok((
        Position::from_raw(r_inertial),
        VelocityVector::from_raw(v_inertial),
    ))
}