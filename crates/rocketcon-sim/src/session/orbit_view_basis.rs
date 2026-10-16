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