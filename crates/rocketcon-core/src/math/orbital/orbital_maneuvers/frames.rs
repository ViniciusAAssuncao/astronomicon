use super::types::ManeuverDeltaV;
use astronomicon_core::units::{Position, Speed, VelocityVector};

pub fn local_to_inertial_delta_v(
    maneuver: ManeuverDeltaV,
    position: Position,
    velocity: VelocityVector,
) -> VelocityVector {
    let r_vec = position.raw();
    let v_vec = velocity.raw();

    let v_mag = v_vec.magnitude();
    if v_mag < 1e-12 || !v_mag.is_finite() {
        return VelocityVector::zero();
    }

    let h_vec = r_vec.cross(&v_vec);
    let h_mag = h_vec.magnitude();

    let u_pro = v_vec / v_mag;
    let u_norm = if h_mag > 1e-12 {
        h_vec / h_mag
    } else {
        u_pro.any_perpendicular()
    };
    let u_rad = u_norm.cross(&u_pro).normalized();

    let dv_inertial = u_pro * maneuver.prograde.value()
        + u_norm * maneuver.normal.value()
        + u_rad * maneuver.radial.value();

    VelocityVector::from_raw(dv_inertial)
}

pub fn inertial_to_local_delta_v(
    delta_v_inertial: VelocityVector,
    position: Position,
    velocity: VelocityVector,
) -> ManeuverDeltaV {
    let r_vec = position.raw();
    let v_vec = velocity.raw();
    let dv_vec = delta_v_inertial.raw();

    let v_mag = v_vec.magnitude();
    if v_mag < 1e-12 || !v_mag.is_finite() {
        return ManeuverDeltaV::zero();
    }

    let h_vec = r_vec.cross(&v_vec);
    let h_mag = h_vec.magnitude();

    let u_pro = v_vec / v_mag;
    let u_norm = if h_mag > 1e-12 {
        h_vec / h_mag
    } else {
        u_pro.any_perpendicular()
    };
    let u_rad = u_norm.cross(&u_pro).normalized();

    let dv_pro = dv_vec.dot(&u_pro);
    let dv_norm = dv_vec.dot(&u_norm);
    let dv_rad = dv_vec.dot(&u_rad);

    ManeuverDeltaV::new(
        Speed::new(dv_pro),
        Speed::new(dv_norm),
        Speed::new(dv_rad),
    )
}