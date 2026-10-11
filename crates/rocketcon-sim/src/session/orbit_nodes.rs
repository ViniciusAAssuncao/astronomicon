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

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::Vector3;
    use rocketcon_core::math::orbital::cartesian_to_osculating_elements;

    fn state(
        inclination: f64,
        eccentricity: f64,
        anomaly: f64,
    ) -> (
        Position,
        VelocityVector,
        GravitationalParameter,
        OsculatingElements,
    ) {
        let p = 1.0 + eccentricity * anomaly.cos();
        let radius = p / (1.0 + eccentricity * anomaly.cos());
        let radial = Vector3::new(
            anomaly.cos(),
            anomaly.sin() * inclination.cos(),
            anomaly.sin() * inclination.sin(),
        );
        let tangent = Vector3::new(
            -anomaly.sin(),
            anomaly.cos() * inclination.cos(),
            anomaly.cos() * inclination.sin(),
        );
        let r = Position::from_raw(radial * radius);
        let v = VelocityVector::from_raw(
            radial * (eccentricity * anomaly.sin() / p.sqrt())
                + tangent * ((1.0 + eccentricity * anomaly.cos()) / p.sqrt()),
        );
        let mu = GravitationalParameter::new(1.0);
        let elements = cartesian_to_osculating_elements(r, v, mu).unwrap();
        (r, v, mu, elements)
    }

    #[test]
    fn inclined_ellipse_has_both_future_nodes() {
        let (r, v, mu, elements) = state(28.5_f64.to_radians(), 0.15, -0.7);
        let nodes = orbit_nodes(&elements, r, v, mu, 100.0);
        assert!((nodes.inclination_rad.unwrap().to_degrees() - 28.5).abs() < 1e-8);
        assert!(nodes.ascending_longitude_rad.unwrap().abs() < 1e-8);
        let a = nodes.next_ascending.unwrap();
        let d = nodes.next_descending.unwrap();
        assert!(a.time_to_seconds > 0.0);
        assert!(d.time_to_seconds > a.time_to_seconds);
        assert!(a.relative_position_m[2].abs() < 1e-10);
        assert!(d.relative_position_m[2].abs() < 1e-10);
        assert!(a.relative_position_m[0] > 0.0 && d.relative_position_m[0] < 0.0);
    }

    #[test]
    fn polar_circular_and_retrograde_are_valid() {
        for inclination in [90.0_f64, 145.0] {
            let (r, v, mu, elements) = state(inclination.to_radians(), 0.0, -0.5);
            let nodes = orbit_nodes(&elements, r, v, mu, 20.0);
            assert!((nodes.inclination_rad.unwrap().to_degrees() - inclination).abs() < 1e-8);
            assert!(nodes.next_ascending.is_some());
            assert!(nodes.next_descending.is_some());
        }
    }

    #[test]
    fn equatorial_has_no_defined_node() {
        let (r, v, mu, elements) = state(0.0, 0.0, -0.5);
        let nodes = orbit_nodes(&elements, r, v, mu, 20.0);
        assert!(nodes.inclination_rad.unwrap().abs() < 1e-8);
        assert!(nodes.ascending_longitude_rad.is_none());
        assert!(nodes.next_ascending.is_none());
        assert!(nodes.next_descending.is_none());
    }

    #[test]
    fn horizon_and_radial_degeneracy_do_not_invent_events() {
        let (r, v, mu, elements) = state(0.6, 0.1, -0.7);
        let limited = orbit_nodes(&elements, r, v, mu, 0.01);
        assert!(limited.next_ascending.is_none());
        assert!(limited.next_descending.is_none());
        let radial = VelocityVector::from_raw(r.raw() * 2.0);
        let invalid = orbit_nodes(&elements, r, radial, mu, 20.0);
        assert!(invalid.inclination_rad.is_none());
        assert!(invalid.ascending_longitude_rad.is_none());
    }

    #[test]
    fn equatorial_reference_differs_from_inertial_xy() {
        let (r, v, mu, elements) = state(0.55, 0.12, -0.7);
        let h = r.raw().cross(&v.raw()).normalized();
        let aligned = OrbitalReferenceFrame::from_pole(h).unwrap();
        let xy = orbit_nodes(&elements, r, v, mu, 100.0);
        let eq = orbit_nodes_in_frame(&elements, r, v, mu, 100.0, aligned);
        assert!((xy.inclination_rad.unwrap() - 0.55).abs() < 1e-8);
        assert!(eq.inclination_rad.unwrap().abs() < 1e-8);
        assert!(eq.ascending_longitude_rad.is_none());
        assert!(eq.next_ascending.is_none() && eq.next_descending.is_none());
    }

    #[test]
    fn tilted_reference_nodes_lie_on_its_equator() {
        let (r, v, mu, elements) = state(0.85, 0.14, -0.4);
        let frame = OrbitalReferenceFrame::from_pole(Vector3::new(0.12, -0.18, 1.0)).unwrap();
        let nodes = orbit_nodes_in_frame(&elements, r, v, mu, 100.0, frame);
        assert!(nodes.inclination_rad.unwrap() > 0.1);
        assert!(nodes.ascending_longitude_rad.is_some());
        for node in [
            nodes.next_ascending.as_ref(),
            nodes.next_descending.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            let pos = Vector3::new(
                node.relative_position_m[0],
                node.relative_position_m[1],
                node.relative_position_m[2],
            );
            assert!(pos.dot(&frame.normal).abs() < 1e-9);
        }
    }

    #[test]
    fn escape_does_not_invent_unreachable_node() {
        let (r, v, mu, elements) = state(0.8, 1.2, -0.3);
        let nodes = orbit_nodes(&elements, r, v, mu, 100.0);
        assert!(nodes.next_ascending.is_some());
        assert!(nodes.next_descending.is_none());
    }
}
