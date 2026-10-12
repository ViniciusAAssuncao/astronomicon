use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{AccelerationVector, Mass, Vector3};

pub fn third_body_perturbation_acceleration(
    vehicle_pos_rel_primary: Vector3,
    third_body_pos_rel_primary: Vector3,
    third_body_mass: Mass,
) -> AccelerationVector {
    let m = third_body_mass.value();
    if m <= 0.0 || !m.is_finite() {
        return AccelerationVector::zero();
    }

    let r_v = vehicle_pos_rel_primary;
    let r_3 = third_body_pos_rel_primary;

    let d_v3 = r_3 - r_v;
    let dist_v3 = d_v3.magnitude();
    let dist_p3 = r_3.magnitude();

    if dist_v3 <= 1e-3 || dist_p3 <= 1e-3 || !dist_v3.is_finite() || !dist_p3.is_finite() {
        return AccelerationVector::zero();
    }

    let g_m = GRAVITATIONAL_CONSTANT * m;
    let term1 = d_v3 * (g_m / (dist_v3 * dist_v3 * dist_v3));
    let term2 = r_3 * (g_m / (dist_p3 * dist_p3 * dist_p3));
    let a = term1 - term2;

    if !a.0.is_finite() || !a.1.is_finite() || !a.2.is_finite() {
        AccelerationVector::zero()
    } else {
        AccelerationVector::from_raw(a)
    }
}

pub fn accumulated_third_body_perturbations(
    vehicle_pos_rel_primary: Vector3,
    third_bodies: &[(Vector3, Mass)],
) -> AccelerationVector {
    let mut total = Vector3::zero();
    for &(r_3, mass) in third_bodies {
        let a = third_body_perturbation_acceleration(vehicle_pos_rel_primary, r_3, mass);
        total = total + a.raw();
    }
    AccelerationVector::from_raw(total)
}
