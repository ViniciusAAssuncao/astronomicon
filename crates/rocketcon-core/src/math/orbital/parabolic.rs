use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::{
    Angle, Duration, GravitationalParameter, Length, Position, Speed, Vector3, VelocityVector,
};

pub fn solve_barker_parabolic_anomaly(parabolic_mean_anomaly: f64) -> f64 {
    let m = parabolic_mean_anomaly;
    if !m.is_finite() {
        return 0.0;
    }
    let q = 1.5 * m;
    let d = q * q + 1.0;
    let sqrt_d = d.sqrt();
    let w_cubed = q + sqrt_d;
    let w = if w_cubed >= 0.0 {
        w_cubed.cbrt()
    } else {
        -(-w_cubed).cbrt()
    };
    if w.abs() < 1e-15 {
        0.0
    } else {
        w - 1.0 / w
    }
}

pub fn true_anomaly_from_parabolic_anomaly(parabolic_anomaly: f64) -> Angle {
    Angle::new(2.0 * parabolic_anomaly.atan())
}

pub fn parabolic_anomaly_from_true_anomaly(true_anomaly: Angle) -> f64 {
    (0.5 * true_anomaly.value()).tan()
}

pub fn parabolic_mean_anomaly(true_anomaly: Angle) -> f64 {
    let b = (0.5 * true_anomaly.value()).tan();
    b + (1.0 / 3.0) * b.powi(3)
}

pub fn parabolic_time_since_periapsis(
    periapsis_distance: Length,
    mu: GravitationalParameter,
    true_anomaly: Angle,
) -> Duration {
    let q = periapsis_distance.value();
    let mu_val = mu.value();
    if q <= 0.0 || mu_val <= 0.0 || !q.is_finite() || !mu_val.is_finite() {
        return Duration::new(0.0);
    }
    let m_p = parabolic_mean_anomaly(true_anomaly);
    let scale = (2.0 * q.powi(3) / mu_val).sqrt();
    Duration::new(m_p * scale)
}

pub fn true_anomaly_from_parabolic_time(
    periapsis_distance: Length,
    mu: GravitationalParameter,
    time_since_periapsis: Duration,
) -> Angle {
    let q = periapsis_distance.value();
    let mu_val = mu.value();
    let dt = time_since_periapsis.value();

    if q <= 0.0 || mu_val <= 0.0 || !q.is_finite() || !mu_val.is_finite() || !dt.is_finite() {
        return Angle::new(0.0);
    }

    let scale = (2.0 * q.powi(3) / mu_val).sqrt();
    if scale <= 0.0 {
        return Angle::new(0.0);
    }

    let m_p = dt / scale;
    let b = solve_barker_parabolic_anomaly(m_p);
    true_anomaly_from_parabolic_anomaly(b)
}

pub fn parabolic_radius(periapsis_distance: Length, true_anomaly: Angle) -> Length {
    let q = periapsis_distance.value();
    let nu = true_anomaly.value();
    if q <= 0.0 || !q.is_finite() || !nu.is_finite() {
        return Length::new(0.0);
    }
    let b = (0.5 * nu).tan();
    Length::new(q * (1.0 + b * b))
}

pub fn parabolic_orbital_speed(mu: GravitationalParameter, radius: Length) -> Speed {
    let mu_val = mu.value();
    let r = radius.value();
    if mu_val <= 0.0 || r <= 0.0 || !mu_val.is_finite() || !r.is_finite() {
        Speed::new(0.0)
    } else {
        Speed::new(((2.0 * mu_val) / r).sqrt())
    }
}

pub fn parabolic_flight_path_angle(true_anomaly: Angle) -> Angle {
    Angle::new(0.5 * true_anomaly.value())
}

pub fn parabolic_state_vectors(
    periapsis_distance: Length,
    true_anomaly: Angle,
    inclination: Angle,
    longitude_of_ascending_node: Angle,
    argument_of_periapsis: Angle,
    mu: GravitationalParameter,
) -> RocketDomainResult<(Position, VelocityVector)> {
    let q = periapsis_distance.value();
    let nu = true_anomaly.value();
    let inc = inclination.value();
    let raan = longitude_of_ascending_node.value();
    let omega = argument_of_periapsis.value();
    let mu_val = mu.value();

    if q <= 0.0 || mu_val <= 0.0 || !q.is_finite() || !mu_val.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "parabolic_elements".to_string(),
            reason: "invalid parameters for parabolic state vectors".to_string(),
        });
    }

    let p = 2.0 * q;
    let denom = 1.0 + nu.cos();
    if denom <= 1e-12 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "true_anomaly".to_string(),
            reason: "true anomaly is at parabolic escape point (180 degrees)".to_string(),
        });
    }

    let r = p / denom;
    let r_pqw = Vector3::new(r * nu.cos(), r * nu.sin(), 0.0);

    let h = (mu_val * p).sqrt();
    let v_factor = mu_val / h;
    let v_pqw = Vector3::new(-v_factor * nu.sin(), v_factor * (1.0 + nu.cos()), 0.0);

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