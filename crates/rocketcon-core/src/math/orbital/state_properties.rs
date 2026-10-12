use crate::math::orbital::types::OsculatingElements;
use astronomicon_core::units::{
    Angle, Duration, GravitationalParameter, Length, Speed, Vector3,
};
use std::f64::consts::TAU;

pub fn specific_angular_momentum(position: Vector3, velocity: Vector3) -> Vector3 {
    position.cross(&velocity)
}

pub fn specific_orbital_energy(position: Vector3, velocity: Vector3, mu: f64) -> f64 {
    let r = position.magnitude();
    let v_sq = velocity.dot(&velocity);
    if r <= 0.0 || !r.is_finite() || !v_sq.is_finite() || !mu.is_finite() {
        0.0
    } else {
        0.5 * v_sq - mu / r
    }
}

pub fn laplace_runge_lenz_vector(position: Vector3, velocity: Vector3, mu: f64) -> Vector3 {
    let r = position.magnitude();
    let v_sq = velocity.dot(&velocity);
    let r_dot_v = position.dot(&velocity);

    if r <= 0.0 || mu <= 0.0 || !r.is_finite() || !mu.is_finite() {
        return Vector3::zero();
    }

    let term1 = position * (v_sq - mu / r);
    let term2 = velocity * r_dot_v;
    (term1 - term2) / mu
}

pub fn semi_major_axis_from_energy(energy: f64, mu: f64) -> Length {
    if !energy.is_finite() || energy.abs() < 1e-15 || mu <= 0.0 || !mu.is_finite() {
        Length::new(f64::INFINITY)
    } else {
        Length::new(-mu / (2.0 * energy))
    }
}

pub fn flight_path_angle(position: Vector3, velocity: Vector3) -> Angle {
    let r_dot_v = position.dot(&velocity);
    let h_vec = position.cross(&velocity);
    let h = h_vec.magnitude();

    if !r_dot_v.is_finite() || !h.is_finite() || (r_dot_v == 0.0 && h == 0.0) {
        Angle::new(0.0)
    } else {
        Angle::new(r_dot_v.atan2(h))
    }
}

pub fn vis_viva_speed(radius: Length, semi_major_axis: Length, mu: GravitationalParameter) -> Speed {
    let r = radius.value();
    let a = semi_major_axis.value();
    let mu_val = mu.value();

    if r <= 0.0 || mu_val <= 0.0 || !r.is_finite() || !mu_val.is_finite() {
        return Speed::new(0.0);
    }

    let inv_a = if a.is_infinite() || !a.is_finite() || a.abs() < 1e-15 {
        0.0
    } else {
        1.0 / a
    };

    let v_sq = mu_val * (2.0 / r - inv_a);
    if v_sq <= 0.0 || !v_sq.is_finite() {
        Speed::new(0.0)
    } else {
        Speed::new(v_sq.sqrt())
    }
}

pub fn orbital_period_if_bound(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
) -> Option<Duration> {
    if elements.eccentricity >= 1.0 {
        return None;
    }
    let a = elements.semi_major_axis.value();
    let mu_val = mu.value();
    if a <= 0.0 || mu_val <= 0.0 || !a.is_finite() || !mu_val.is_finite() {
        None
    } else {
        Some(Duration::new(TAU * (a.powi(3) / mu_val).sqrt()))
    }
}

pub fn periapsis_speed(elements: &OsculatingElements, mu: GravitationalParameter) -> Speed {
    vis_viva_speed(elements.periapsis_distance, elements.semi_major_axis, mu)
}

pub fn apoapsis_speed(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
) -> Option<Speed> {
    elements
        .apoapsis_distance
        .map(|r_apo| vis_viva_speed(r_apo, elements.semi_major_axis, mu))
}