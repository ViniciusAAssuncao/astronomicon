use crate::units::constants::{GRAVITATIONAL_CONSTANT, SPEED_OF_LIGHT, THORNE_SPIN_LIMIT};
use crate::units::{AngularVelocity, Duration, Length, Mass};
use std::f64::consts::TAU;

pub fn gravitational_radius(mass: Mass) -> Length {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Length::new(0.0);
    }
    let rg = (GRAVITATIONAL_CONSTANT * m) / (SPEED_OF_LIGHT * SPEED_OF_LIGHT);
    Length::new(rg)
}

pub fn schwarzschild_radius(mass: Mass) -> Length {
    Length::new(2.0 * gravitational_radius(mass).value())
}

pub fn event_horizon_radius(mass: Mass, dimensionless_spin: f64) -> Length {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return Length::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let term = (1.0 - a_star * a_star).max(0.0).sqrt();
    Length::new(rg * (1.0 + term))
}

pub fn horizon_angular_velocity(mass: Mass, dimensionless_spin: f64) -> AngularVelocity {
    let rg = gravitational_radius(mass).value();
    if rg <= 0.0 {
        return AngularVelocity::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    if a_star <= 0.0 {
        return AngularVelocity::new(0.0);
    }
    let term = (1.0 - a_star * a_star).max(0.0).sqrt();
    let r_plus = rg * (1.0 + term);
    let omega_h = (a_star * SPEED_OF_LIGHT) / (2.0 * r_plus);
    AngularVelocity::new(omega_h)
}

pub fn horizon_rotation_period(mass: Mass, dimensionless_spin: f64) -> Option<Duration> {
    let omega = horizon_angular_velocity(mass, dimensionless_spin).value();
    if omega <= 0.0 || !omega.is_finite() {
        None
    } else {
        Some(Duration::new(TAU / omega))
    }
}

pub fn dimensionless_spin_from_angular_velocity(
    mass: Mass,
    angular_velocity: AngularVelocity,
) -> f64 {
    let rg = gravitational_radius(mass).value();
    let omega = angular_velocity.value().abs();
    if rg <= 0.0 || omega <= 0.0 || !rg.is_finite() || !omega.is_finite() {
        return 0.0;
    }
    let w = (omega * rg) / SPEED_OF_LIGHT;
    if w >= 0.5 {
        return THORNE_SPIN_LIMIT;
    }
    let a_star = (4.0 * w) / (1.0 + 4.0 * w * w);
    a_star.clamp(0.0, THORNE_SPIN_LIMIT)
}

pub fn dimensionless_spin_from_rotation_period(
    mass: Mass,
    rotation_period: Duration,
) -> f64 {
    let period = rotation_period.value();
    if period <= 0.0 || !period.is_finite() {
        return 0.0;
    }
    let omega_h = AngularVelocity::new(TAU / period);
    dimensionless_spin_from_angular_velocity(mass, omega_h)
}

pub fn dimensionless_spin(mass: Mass, rotation_period: Duration) -> f64 {
    dimensionless_spin_from_rotation_period(mass, rotation_period)
}