use super::OrbitalReferenceFrame;
use astronomicon_core::units::{Angle, GravitationalParameter, Position, Vector3, VelocityVector};
use rocketcon_core::math::orbital::OsculatingElements;
use rocketcon_core::math::orbital::state_properties::laplace_runge_lenz_vector;
use rocketcon_core::math::orbital::trajectory_prediction::time_between_true_anomalies;
use std::f64::consts::TAU;

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitNode {
    pub time_to_seconds: f64,
    pub relative_position_m: [f64; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitNodes {
    pub inclination_rad: Option<f64>,
    pub ascending_longitude_rad: Option<f64>,
    pub next_ascending: Option<OrbitNode>,
    pub next_descending: Option<OrbitNode>,
}

pub fn orbit_nodes(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    horizon: f64,
) -> OrbitNodes {
    orbit_nodes_in_frame(
        elements,
        position,
        velocity,
        mu,
        horizon,
        OrbitalReferenceFrame::inertial_xy(),
    )
}

pub fn orbit_nodes_in_frame(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    horizon: f64,
    frame: OrbitalReferenceFrame,
) -> OrbitNodes {
    let mut output = OrbitNodes {
        inclination_rad: None,
        ascending_longitude_rad: None,
        next_ascending: None,
        next_descending: None,
    };
    let r = position.raw();
    let v = velocity.raw();
    let r_norm = r.magnitude();
    let v_norm = v.magnitude();
    let h = r.cross(&v);
    let h_norm = h.magnitude();
    if !r_norm.is_finite()
        || !v_norm.is_finite()
        || !h_norm.is_finite()
        || r_norm <= 0.0
        || v_norm <= 0.0
        || h_norm / r_norm / v_norm <= 1e-9
    {
        return output;
    }
    let normal = h / h_norm;
    let inclination = normal.dot(&frame.normal).clamp(-1.0, 1.0).acos();
    if !inclination.is_finite() {
        return output;
    }
    output.inclination_rad = Some(inclination);
    let line = frame.normal.cross(&normal);
    let line_length = line.magnitude();
    if !line_length.is_finite() || line_length <= 1e-7 {
        return output;
    }
    let ascending = line / line_length;
    output.ascending_longitude_rad = Some(
        ascending
            .dot(&frame.vertical)
            .atan2(ascending.dot(&frame.horizontal))
            .rem_euclid(TAU),
    );
    if !horizon.is_finite() || horizon <= 0.0 || mu.value() <= 0.0 {
        return output;
    }
    let eccentricity = laplace_runge_lenz_vector(r, v, mu.value());
    output.next_ascending = find_node(
        elements,
        r,
        eccentricity,
        mu,
        h_norm,
        normal,
        ascending,
        horizon,
    );
    output.next_descending = find_node(
        elements,
        r,
        eccentricity,
        mu,
        h_norm,
        normal,
        ascending * -1.0,
        horizon,
    );
    output
}

fn find_node(
    elements: &OsculatingElements,
    position: Vector3,
    eccentricity_vector: Vector3,
    mu: GravitationalParameter,
    h_norm: f64,
    normal: Vector3,
    direction: Vector3,
    horizon: f64,
) -> Option<OrbitNode> {
    let e = elements.eccentricity;
    if !e.is_finite() || e < 0.0 {
        return None;
    }
    let p = h_norm * h_norm / mu.value();
    let denominator = 1.0 + eccentricity_vector.dot(&direction);
    if !p.is_finite() || !denominator.is_finite() || denominator <= 1e-10 {
        return None;
    }
    let radius = p / denominator;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let eccentricity_axis = eccentricity_vector.magnitude();
    let dt = if eccentricity_axis >= 1e-8 {
        let peri = eccentricity_vector / eccentricity_axis;
        let transverse = normal.cross(&peri);
        let target_nu = direction.dot(&transverse).atan2(direction.dot(&peri));
        let target_nu = if e < 1.0 {
            target_nu.rem_euclid(TAU)
        } else {
            target_nu
        };
        time_between_true_anomalies(elements, mu, elements.true_anomaly, Angle::new(target_nu))
            .ok()?
            .value()
    } else if e < 1.0 {
        let major = position / position.magnitude();
        let transverse = normal.cross(&major);
        let phase = direction
            .dot(&transverse)
            .atan2(direction.dot(&major))
            .rem_euclid(TAU);
        let n = (mu.value() / elements.semi_major_axis.value().powi(3)).sqrt();
        if !n.is_finite() || n <= 0.0 {
            return None;
        }
        phase / n
    } else {
        return None;
    };
    if !dt.is_finite() || dt < 0.0 || dt > horizon * (1.0 + 1e-8) {
        return None;
    }
    let point = direction * radius;
    Some(OrbitNode {
        time_to_seconds: dt.min(horizon),
        relative_position_m: [point.0, point.1, point.2],
    })
}