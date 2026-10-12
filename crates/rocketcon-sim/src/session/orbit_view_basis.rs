use astronomicon_core::units::{Position, Vector3, VelocityVector};

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitViewBasis {
    pub horizontal: [f64; 3],
    pub vertical: [f64; 3],
}

pub fn orbit_view_basis(position: Position, velocity: VelocityVector) -> Option<OrbitViewBasis> {
    let r = position.raw();
    let v = velocity.raw();
    let r_length = r.magnitude();
    let v_length = v.magnitude();
    if !r_length.is_finite() || !v_length.is_finite() ||
        r_length <= 0.0 || v_length <= 0.0 {
        return None;
    }
    let h = r.cross(&v);
    let h_length = h.magnitude();
    if !h_length.is_finite() || h_length <= 0.0 ||
        h_length / r_length / v_length <= 1e-9 {
        return None;
    }
    let normal = h / h_length;
    let anchor = if normal.0.abs() < 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let projected = anchor - normal * anchor.dot(&normal);
    let projected_length = projected.magnitude();
    if !projected_length.is_finite() || projected_length <= 0.0 {
        return None;
    }
    let horizontal = projected / projected_length;
    let vertical = normal.cross(&horizontal);
    if ![horizontal.0, horizontal.1, horizontal.2,
        vertical.0, vertical.1, vertical.2]
        .iter().all(|value| value.is_finite()) {
        return None;
    }
    Some(OrbitViewBasis {
        horizontal: [horizontal.0, horizontal.1, horizontal.2],
        vertical: [vertical.0, vertical.1, vertical.2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(position: [f64; 3], velocity: [f64; 3]) -> OrbitViewBasis {
        let r = Position::from_raw(Vector3::new(position[0], position[1], position[2]));
        let v = VelocityVector::from_raw(Vector3::new(velocity[0], velocity[1], velocity[2]));
        let basis = orbit_view_basis(r, v).expect("valid orbital plane");
        let horizontal = Vector3::new(basis.horizontal[0], basis.horizontal[1], basis.horizontal[2]);
        let vertical = Vector3::new(basis.vertical[0], basis.vertical[1], basis.vertical[2]);
        let normal = horizontal.cross(&vertical);
        let expected = r.raw().cross(&v.raw()).normalized();
        assert!((horizontal.magnitude() - 1.0).abs() < 1e-12);
        assert!((vertical.magnitude() - 1.0).abs() < 1e-12);
        assert!(horizontal.dot(&vertical).abs() < 1e-12);
        assert!((normal.dot(&expected) - 1.0).abs() < 1e-12);
        assert!(horizontal.dot(&expected).abs() < 1e-12);
        assert!(vertical.dot(&expected).abs() < 1e-12);
        basis
    }

    #[test]
    fn supports_equatorial_and_retrograde() {
        let direct = check([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert_eq!(direct.horizontal, [1.0, 0.0, 0.0]);
        assert_eq!(direct.vertical, [0.0, 1.0, 0.0]);
        let retro = check([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]);
        assert_eq!(retro.horizontal, [1.0, 0.0, 0.0]);
        assert_eq!(retro.vertical, [0.0, -1.0, 0.0]);
    }

    #[test]
    fn supports_inclined_polar_circular_and_open_states() {
        check([1.0, 0.0, 0.0], [0.0, 0.5, 0.8660254038]);
        check([0.0, 1.0, 0.0], [0.0, 0.0, 2.0]);
        check([5.0, -2.0, 4.0], [2.0, 10.0, 5.0]);
        check([1.0, 0.0, 0.0], [0.0, 100.0, 0.0]);
    }

    #[test]
    fn rejects_radial_zero_and_non_finite_states() {
        for (position, velocity) in [
            ([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
            ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
            ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
            ([1.0, 0.0, 0.0], [2.0, 1e-12, 0.0]),
            ([f64::INFINITY, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ] {
            let r = Position::from_raw(Vector3::new(position[0], position[1], position[2]));
            let v = VelocityVector::from_raw(Vector3::new(velocity[0], velocity[1], velocity[2]));
            assert!(orbit_view_basis(r, v).is_none());
        }
    }
}
