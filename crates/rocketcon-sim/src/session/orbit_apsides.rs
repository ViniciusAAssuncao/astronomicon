use astronomicon_core::units::{Angle, GravitationalParameter, Position, VelocityVector};
use rocketcon_core::math::orbital::trajectory_prediction::time_between_true_anomalies;
use rocketcon_core::math::orbital::state_properties::laplace_runge_lenz_vector;
use rocketcon_core::math::orbital::OsculatingElements;
use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitApsis {
    pub time_to_seconds: f64,
    pub relative_position_m: [f64; 3],
}

pub fn next_apsis(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    anomaly: Angle,
    horizon_seconds: f64,
) -> Option<OrbitApsis> {
    if !(1e-5..1.0).contains(&elements.eccentricity) {
        return None;
    }
    let dt = time_between_true_anomalies(elements, mu, elements.true_anomaly, anomaly)
        .ok()?
        .value();
    let boundary_tolerance = horizon_seconds * 1e-8;
    if !dt.is_finite() || dt < 0.0 || dt > horizon_seconds + boundary_tolerance {
        return None;
    }
    let dt = dt.min(horizon_seconds);
    let r = position.raw();
    let v = velocity.raw();
    let eccentricity_axis = laplace_runge_lenz_vector(r, v, mu.value());
    let eccentricity = eccentricity_axis.magnitude();
    let angular_momentum = r.cross(&v);
    let h = angular_momentum.magnitude();
    if !eccentricity.is_finite() || eccentricity < 1e-5 || !h.is_finite() || h <= 0.0 {
        return None;
    }
    let periapsis_direction = eccentricity_axis / eccentricity;
    let transverse_direction = angular_momentum.cross(&periapsis_direction) / h;
    let denominator = 1.0 + elements.eccentricity * anomaly.value().cos();
    if !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }
    let radius = (h * h / mu.value()) / denominator;
    let point = (periapsis_direction * anomaly.value().cos()
        + transverse_direction * anomaly.value().sin()) * radius;
    if ![point.0, point.1, point.2]
        .iter()
        .all(|value| value.is_finite())
    {
        return None;
    }
    Some(OrbitApsis {
        time_to_seconds: dt,
        relative_position_m: [point.0, point.1, point.2],
    })
}

pub fn next_periapsis(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    horizon_seconds: f64,
) -> Option<OrbitApsis> {
    next_apsis(
        elements,
        position,
        velocity,
        mu,
        Angle::new(0.0),
        horizon_seconds,
    )
}

pub fn next_apoapsis(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    horizon_seconds: f64,
) -> Option<OrbitApsis> {
    next_apsis(
        elements,
        position,
        velocity,
        mu,
        Angle::new(PI),
        horizon_seconds,
    )
}