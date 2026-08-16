use crate::units::{AngularVelocity, Duration, GravitationalParameter, Length, Speed};
use std::f64::consts::PI;

pub fn orbital_period(
    semi_major_axis: Length,
    mu: GravitationalParameter,
) -> Option<Duration> {
    if semi_major_axis.value() <= 0.0 {
        None
    } else {
        Some(Duration::new(
            2.0 * PI * (semi_major_axis.value().powi(3) / mu.value()).sqrt(),
        ))
    }
}

pub fn mean_motion(
    semi_major_axis: Length,
    mu: GravitationalParameter,
) -> AngularVelocity {
    AngularVelocity::new(
        (mu.value() / semi_major_axis.value().abs().powi(3)).sqrt(),
    )
}

pub fn orbital_speed(
    mu: GravitationalParameter,
    radius: Length,
    semi_major_axis: Length,
) -> Speed {
    if radius.value() <= 0.0 {
        Speed::new(0.0)
    } else {
        let v_sq = mu.value() * (2.0 / radius.value() - 1.0 / semi_major_axis.value());
        Speed::new(v_sq.max(0.0).sqrt())
    }
}