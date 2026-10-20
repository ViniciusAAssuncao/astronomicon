use astronomicon_core::units::{GravitationalParameter, Position, VelocityVector};
use rocketcon_core::math::orbital::{OrbitType, OsculatingElements};

const PAST_CURVE_SEGMENTS: usize = 192;

pub(super) fn open_osculating_past(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    elements: &OsculatingElements,
) -> Vec<[f64; 3]> {
    if !matches!(elements.orbit_type(), OrbitType::Hyperbolic | OrbitType::Parabolic)
        || !elements.eccentricity.is_finite()
        || elements.eccentricity < 1.0
        || !mu.value().is_finite()
        || mu.value() <= 0.0
    {
        return Vec::new();
    }
    let radial_vector = position.raw();
    let r = radial_vector.magnitude();
    let h = radial_vector.cross(&velocity.raw());
    let h_norm = h.magnitude();
    if !r.is_finite() || r <= 0.0 || !h_norm.is_finite() || h_norm <= 0.0 {
        return Vec::new();
    }
    let semi_latus = h_norm * h_norm / mu.value();
    let limit_radius = r * 2.5;
    if !semi_latus.is_finite() || semi_latus <= 0.0
        || !limit_radius.is_finite() || limit_radius <= 0.0
    {
        return Vec::new();
    }
    let current = elements.true_anomaly.value();
    if !current.is_finite() {
        return Vec::new();
    }
    let current_anomaly = current.sin().atan2(current.cos());
    let eccentricity = elements.eccentricity;
    let near_asymptote = ((semi_latus / limit_radius - 1.0) / eccentricity)
        .clamp(-1.0, 1.0);
    let start_anomaly = -near_asymptote.acos();
    if current_anomaly <= start_anomaly {
        return Vec::new();
    }
    let radial = radial_vector / r;
    let tangent = (h / h_norm).cross(&radial);
    let major = radial * current_anomaly.cos() - tangent * current_anomaly.sin();
    let minor = radial * current_anomaly.sin() + tangent * current_anomaly.cos();
    let mut points = Vec::with_capacity(PAST_CURVE_SEGMENTS + 1);
    for index in 0..PAST_CURVE_SEGMENTS {
        let fraction = index as f64 / PAST_CURVE_SEGMENTS as f64;
        let anomaly = start_anomaly + (current_anomaly - start_anomaly) * fraction;
        let denominator = 1.0 + eccentricity * anomaly.cos();
        if denominator <= 0.0 {
            return Vec::new();
        }
        let distance = semi_latus / denominator;
        let point = major * (distance * anomaly.cos())
            + minor * (distance * anomaly.sin());
        if ![point.0, point.1, point.2].iter().all(|v| v.is_finite()) {
            return Vec::new();
        }
        points.push([point.0, point.1, point.2]);
    }
    points.push([radial_vector.0, radial_vector.1, radial_vector.2]);
    points
}
