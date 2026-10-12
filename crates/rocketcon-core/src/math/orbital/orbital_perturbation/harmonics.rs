use super::types::ZonalHarmonics;
use crate::math::orbital::types::OsculatingElements;
use astronomicon_core::math::kepler::mean_motion;
use astronomicon_core::units::{
    AccelerationVector,
    AngularVelocity,
    GravitationalParameter,
    Length,
    Position,
    Quaternion,
};

pub fn j2_perturbation_acceleration(
    mu: GravitationalParameter,
    eq_radius: Length,
    j2: f64,
    body_fixed_position: Position
) -> AccelerationVector {
    let pos = body_fixed_position.raw();
    let r_sq = pos.dot(&pos);
    let r = r_sq.sqrt();
    let mu_val = mu.value();
    let req = eq_radius.value();

    if r <= 1e-3 || !r.is_finite() || mu_val <= 0.0 || j2 == 0.0 {
        return AccelerationVector::zero();
    }

    let u = pos.2 / r;
    let factor = (-1.5 * j2 * mu_val * req * req) / (r_sq * r_sq);
    let u_sq = u * u;

    let ax = factor * (pos.0 / r) * (1.0 - 5.0 * u_sq);
    let ay = factor * (pos.1 / r) * (1.0 - 5.0 * u_sq);
    let az = factor * (pos.2 / r) * (3.0 - 5.0 * u_sq);

    AccelerationVector::from_components(ax, ay, az)
}

pub fn j3_perturbation_acceleration(
    mu: GravitationalParameter,
    eq_radius: Length,
    j3: f64,
    body_fixed_position: Position
) -> AccelerationVector {
    let pos = body_fixed_position.raw();
    let r_sq = pos.dot(&pos);
    let r = r_sq.sqrt();
    let mu_val = mu.value();
    let req = eq_radius.value();

    if r <= 1e-3 || !r.is_finite() || mu_val <= 0.0 || j3 == 0.0 {
        return AccelerationVector::zero();
    }

    let u = pos.2 / r;
    let u_sq = u * u;
    let r_fifth = r_sq * r_sq * r;
    let factor_xy = (-2.5 * j3 * mu_val * req.powi(3)) / r_fifth;
    let factor_z = (0.5 * j3 * mu_val * req.powi(3)) / (r_sq * r_sq);

    let ax = factor_xy * (pos.0 / r) * (3.0 * u - 7.0 * u * u_sq);
    let ay = factor_xy * (pos.1 / r) * (3.0 * u - 7.0 * u * u_sq);
    let az = factor_z * (35.0 * u_sq * u_sq - 30.0 * u_sq + 3.0);

    AccelerationVector::from_components(ax, ay, az)
}

pub fn j4_perturbation_acceleration(
    mu: GravitationalParameter,
    eq_radius: Length,
    j4: f64,
    body_fixed_position: Position
) -> AccelerationVector {
    let pos = body_fixed_position.raw();
    let r_sq = pos.dot(&pos);
    let r = r_sq.sqrt();
    let mu_val = mu.value();
    let req = eq_radius.value();

    if r <= 1e-3 || !r.is_finite() || mu_val <= 0.0 || j4 == 0.0 {
        return AccelerationVector::zero();
    }

    let u = pos.2 / r;
    let u_sq = u * u;
    let r_sixth = r_sq * r_sq * r_sq;
    let factor_xy = (-1.875 * j4 * mu_val * req.powi(4)) / r_sixth;
    let factor_z = (0.625 * j4 * mu_val * req.powi(4)) / r_sixth;

    let ax = factor_xy * (pos.0 / r) * (1.0 - 14.0 * u_sq + 21.0 * u_sq * u_sq);
    let ay = factor_xy * (pos.1 / r) * (1.0 - 14.0 * u_sq + 21.0 * u_sq * u_sq);
    let az = factor_z * (pos.2 / r) * (15.0 - 70.0 * u_sq + 63.0 * u_sq * u_sq);

    AccelerationVector::from_components(ax, ay, az)
}

pub fn zonal_harmonics_acceleration_body(
    mu: GravitationalParameter,
    eq_radius: Length,
    harmonics: &ZonalHarmonics,
    body_fixed_position: Position
) -> AccelerationVector {
    let a_j2 = j2_perturbation_acceleration(mu, eq_radius, harmonics.j2, body_fixed_position);
    let a_j3 = j3_perturbation_acceleration(mu, eq_radius, harmonics.j3, body_fixed_position);
    let a_j4 = j4_perturbation_acceleration(mu, eq_radius, harmonics.j4, body_fixed_position);
    a_j2 + a_j3 + a_j4
}

pub fn zonal_harmonics_acceleration_inertial(
    mu: GravitationalParameter,
    eq_radius: Length,
    harmonics: &ZonalHarmonics,
    body_position_inertial: Position,
    vehicle_position_inertial: Position,
    body_orientation: Quaternion
) -> AccelerationVector {
    if harmonics.is_zero() {
        return AccelerationVector::zero();
    }
    let r_rel_inertial = vehicle_position_inertial.raw() - body_position_inertial.raw();
    let r_bf = body_orientation.inverse().rotate_vector(r_rel_inertial);
    let a_bf = zonal_harmonics_acceleration_body(
        mu,
        eq_radius,
        harmonics,
        Position::from_raw(r_bf)
    );
    let a_inertial = body_orientation.rotate_vector(a_bf.raw());
    AccelerationVector::from_raw(a_inertial)
}

pub fn secular_nodal_precession_rate(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    eq_radius: Length,
    harmonics: &ZonalHarmonics
) -> AngularVelocity {
    let a = elements.semi_major_axis.value();
    let e = elements.eccentricity;
    let inc = elements.inclination.value();
    let req = eq_radius.value();

    if a <= 0.0 || e >= 1.0 || req <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let p = a * (1.0 - e * e);
    if p <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let n = mean_motion(elements.semi_major_axis, mu).value();
    let req_p = req / p;
    let cos_i = inc.cos();

    let d_raan_j2 = -1.5 * harmonics.j2 * req_p * req_p * n * cos_i;
    let d_raan_j4 = if harmonics.j4 != 0.0 {
        let req_p_4 = req_p.powi(4);
        0.9375 *
            harmonics.j4 *
            req_p_4 *
            n *
            cos_i *
            (1.0 + 1.5 * e * e) *
            (7.0 * cos_i * cos_i - 3.0)
    } else {
        0.0
    };

    AngularVelocity::new(d_raan_j2 + d_raan_j4)
}

pub fn secular_apsidal_precession_rate(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    eq_radius: Length,
    harmonics: &ZonalHarmonics
) -> AngularVelocity {
    let a = elements.semi_major_axis.value();
    let e = elements.eccentricity;
    let inc = elements.inclination.value();
    let req = eq_radius.value();

    if a <= 0.0 || e >= 1.0 || req <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let p = a * (1.0 - e * e);
    if p <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let n = mean_motion(elements.semi_major_axis, mu).value();
    let req_p = req / p;
    let cos_i = inc.cos();
    let cos_sq = cos_i * cos_i;

    let d_w_j2 = 0.75 * harmonics.j2 * req_p * req_p * n * (5.0 * cos_sq - 1.0);
    let d_w_j4 = if harmonics.j4 != 0.0 {
        let req_p_4 = req_p.powi(4);
        -0.46875 *
            harmonics.j4 *
            req_p_4 *
            n *
            ((1.0 + 1.5 * e * e) * (3.0 - 30.0 * cos_sq + 35.0 * cos_sq * cos_sq) +
                5.0 * e * e * (1.0 - 7.0 * cos_sq))
    } else {
        0.0
    };

    AngularVelocity::new(d_w_j2 + d_w_j4)
}

pub fn secular_mean_motion_j2_correction(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    eq_radius: Length,
    j2: f64
) -> AngularVelocity {
    let a = elements.semi_major_axis.value();
    let e = elements.eccentricity;
    let inc = elements.inclination.value();
    let req = eq_radius.value();

    if a <= 0.0 || e >= 1.0 || req <= 0.0 || j2 == 0.0 {
        return AngularVelocity::new(0.0);
    }

    let p = a * (1.0 - e * e);
    let n = mean_motion(elements.semi_major_axis, mu).value();
    let req_p = req / p;
    let cos_i = inc.cos();
    let eta = (1.0 - e * e).sqrt();

    let d_m = 0.75 * j2 * req_p * req_p * n * eta * (3.0 * cos_i * cos_i - 1.0);
    AngularVelocity::new(d_m)
}
