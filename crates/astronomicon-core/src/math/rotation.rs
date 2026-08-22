use crate::units::{Angle, AngularVelocity, Duration, Length};
use std::f64::consts::TAU;

pub fn angular_velocity_from_rotation_period(rotation_period: Duration) -> AngularVelocity {
    let period = rotation_period.value();
    if period <= 0.0 || !period.is_finite() {
        AngularVelocity::new(0.0)
    } else {
        AngularVelocity::new(TAU / period)
    }
}

pub fn coriolis_parameter(angular_velocity: AngularVelocity, latitude: Angle) -> AngularVelocity {
    let omega = angular_velocity.value();
    let phi = latitude.value();
    if !omega.is_finite() || !phi.is_finite() {
        AngularVelocity::new(0.0)
    } else {
        AngularVelocity::new(2.0 * omega * phi.sin())
    }
}

pub fn rossby_beta_parameter(
    angular_velocity: AngularVelocity,
    latitude: Angle,
    radius: Length,
) -> f64 {
    let omega = angular_velocity.value();
    let phi = latitude.value();
    let r = radius.value();
    if !omega.is_finite() || !phi.is_finite() || !r.is_finite() || r <= 0.0 {
        0.0
    } else {
        (2.0 * omega * phi.cos()) / r
    }
}