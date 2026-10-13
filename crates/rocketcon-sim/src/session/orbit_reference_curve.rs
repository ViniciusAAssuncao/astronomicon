use astronomicon_core::units::{GravitationalParameter, Length, Position, VelocityVector};
use rocketcon_core::math::orbital::{OrbitType, OsculatingElements};

const CURVE_SEGMENTS: usize = 192;

pub fn closed_osculating_reference(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    elements: &OsculatingElements,
    body_radius: Option<Length>,
) -> Option<Vec<[f64; 3]>> {
    if !matches!(
        elements.orbit_type(),
        OrbitType::Circular | OrbitType::Elliptic
    ) || !elements.eccentricity.is_finite()
        || !(0.0..1.0).contains(&elements.eccentricity)
        || !mu.value().is_finite()
        || mu.value() <= 0.0
    {
        return None;
    }
    let radius = body_radius?.value();
    if !radius.is_finite() || radius <= 0.0 || elements.periapsis_distance.value() <= radius {
        return None;
    }
    let h = position.raw().cross(&velocity.raw());
    let h_magnitude = h.magnitude();
    let r = position.raw().magnitude();
    if !r.is_finite() || r <= 0.0 || !h_magnitude.is_finite() || h_magnitude <= 0.0 {
        return None;
    }
    let radial = position.raw() / r;
    let tangent = (h / h_magnitude).cross(&radial);
    let anomaly = elements.true_anomaly.value();
    let major = radial * anomaly.cos() - tangent * anomaly.sin();
    let minor = radial * anomaly.sin() + tangent * anomaly.cos();
    let semi_latus_rectum = h_magnitude * h_magnitude / mu.value();
    if !semi_latus_rectum.is_finite() || semi_latus_rectum <= 0.0 {
        return None;
    }
    let mut points = Vec::with_capacity(CURVE_SEGMENTS + 1);
    for index in 0..CURVE_SEGMENTS {
        let theta = std::f64::consts::TAU * index as f64 / CURVE_SEGMENTS as f64;
        let distance = semi_latus_rectum / (1.0 + elements.eccentricity * theta.cos());
        let point = major * (distance * theta.cos()) + minor * (distance * theta.sin());
        if ![point.0, point.1, point.2]
            .iter()
            .all(|value| value.is_finite())
        {
            return None;
        }
        points.push([point.0, point.1, point.2]);
    }
    points.push(*points.first()?);
    Some(points)
}
