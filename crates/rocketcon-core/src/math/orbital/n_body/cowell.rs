use super::types::CowellPerturbationConfig;
use astronomicon_core::math::gravity::oblateness::j2_gravitational_acceleration;
use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{AccelerationVector, Position, Vector3};

pub fn cowell_acceleration(
    vehicle_rel_primary: Vector3,
    config: &CowellPerturbationConfig,
) -> Vector3 {
    let r_p = vehicle_rel_primary;
    let r_mag = r_p.magnitude();
    let mu_p = config.primary_body.gravitational_parameter().value();

    if r_mag < 1.0 || mu_p <= 0.0 {
        return Vector3::zero();
    }

    let a_primary = -r_p * (mu_p / (r_mag * r_mag * r_mag));

    let a_j2 = match config.primary_body.j2 {
        Some(j2_val) if j2_val != 0.0 => {
            let j2_acc = j2_gravitational_acceleration(
                config.primary_body.gravitational_parameter(),
                config.primary_body.radius,
                j2_val,
                Position::from_raw(r_p),
            );
            j2_acc.raw()
        }
        _ => Vector3::zero(),
    };

    let mut a_third = Vector3::zero();
    let r_prim_world = config.primary_body.position.raw();
    let r_sc_world = r_prim_world + r_p;

    for body in &config.perturbing_bodies {
        let m_3 = body.mass.value();
        if m_3 <= 0.0 {
            continue;
        }

        let r_3_world = body.position.raw();
        let d_v3 = r_3_world - r_sc_world;
        let d_p3 = r_3_world - r_prim_world;

        let dist_v3 = d_v3.magnitude();
        let dist_p3 = d_p3.magnitude();

        if dist_v3 < 10.0 || dist_p3 < 10.0 {
            continue;
        }

        let gm = GRAVITATIONAL_CONSTANT * m_3;
        let direct = d_v3 * (gm / (dist_v3 * dist_v3 * dist_v3));
        let indirect = d_p3 * (gm / (dist_p3 * dist_p3 * dist_p3));

        a_third = a_third + direct - indirect;
    }

    a_primary + a_j2 + a_third
}

pub fn cowell_acceleration_vector(
    vehicle_rel_primary: Vector3,
    config: &CowellPerturbationConfig,
) -> AccelerationVector {
    AccelerationVector::from_raw(cowell_acceleration(vehicle_rel_primary, config))
}

pub fn perturbation_to_primary_ratio(
    vehicle_rel_primary: Vector3,
    config: &CowellPerturbationConfig,
) -> f64 {
    let r_mag = vehicle_rel_primary.magnitude();
    let mu_p = config.primary_body.gravitational_parameter().value();
    if r_mag < 1.0 || mu_p <= 0.0 {
        return 0.0;
    }

    let a_prim_mag = mu_p / (r_mag * r_mag);
    let total_acc = cowell_acceleration(vehicle_rel_primary, config);
    let a_prim_vec = -vehicle_rel_primary * (mu_p / (r_mag * r_mag * r_mag));
    let pert_vec = total_acc - a_prim_vec;

    pert_vec.magnitude() / a_prim_mag.max(1e-15)
}