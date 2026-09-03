use super::frames::inertial_to_local_delta_v;
use super::types::ManeuverDeltaV;
use astronomicon_core::units::{GravitationalParameter, Position, VelocityVector};

pub fn circularization_delta_v(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
) -> VelocityVector {
    let r_vec = position.raw();
    let v_vec = velocity.raw();
    let mu_val = mu.value();

    let r_mag = r_vec.magnitude();
    if r_mag < 1e-12 || mu_val <= 0.0 || !r_mag.is_finite() || !mu_val.is_finite() {
        return VelocityVector::zero();
    }

    let h_vec = r_vec.cross(&v_vec);
    let h_mag = h_vec.magnitude();

    let u_circ = if h_mag > 1e-12 {
        let u_h = h_vec / h_mag;
        let u_r = r_vec / r_mag;
        u_h.cross(&u_r).normalized()
    } else {
        let u_r = r_vec / r_mag;
        u_r.any_perpendicular()
    };

    let v_circ_mag = (mu_val / r_mag).sqrt();
    let v_target = u_circ * v_circ_mag;
    let dv = v_target - v_vec;

    VelocityVector::from_raw(dv)
}

pub fn circularization_maneuver(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
) -> ManeuverDeltaV {
    let dv_inertial = circularization_delta_v(position, velocity, mu);
    inertial_to_local_delta_v(dv_inertial, position, velocity)
}