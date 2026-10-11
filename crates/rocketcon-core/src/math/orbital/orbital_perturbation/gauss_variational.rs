use super::types::GaussVariationalRates;
use crate::math::orbital::types::OsculatingElements;
use astronomicon_core::units::{
    AccelerationVector, AngularVelocity, GravitationalParameter, Position, Speed, Vector3,
    VelocityVector,
};

pub fn inertial_to_rsw_acceleration(
    position: Vector3,
    velocity: Vector3,
    acceleration_inertial: Vector3,
) -> Vector3 {
    let r_mag = position.magnitude();
    if r_mag < 1e-12 {
        return Vector3::zero();
    }

    let u_r = position / r_mag;
    let h_vec = position.cross(&velocity);
    let h_mag = h_vec.magnitude();

    if h_mag < 1e-12 {
        let u_perp = u_r.any_perpendicular();
        let u_norm = u_r.cross(&u_perp).normalized();
        return Vector3::new(
            acceleration_inertial.dot(&u_r),
            acceleration_inertial.dot(&u_perp),
            acceleration_inertial.dot(&u_norm),
        );
    }

    let u_h = h_vec / h_mag;
    let u_theta = u_h.cross(&u_r).normalized();

    Vector3::new(
        acceleration_inertial.dot(&u_r),
        acceleration_inertial.dot(&u_theta),
        acceleration_inertial.dot(&u_h),
    )
}

pub fn rsw_to_inertial_acceleration(
    position: Vector3,
    velocity: Vector3,
    acceleration_rsw: Vector3,
) -> Vector3 {
    let r_mag = position.magnitude();
    if r_mag < 1e-12 {
        return Vector3::zero();
    }

    let u_r = position / r_mag;
    let h_vec = position.cross(&velocity);
    let h_mag = h_vec.magnitude();

    if h_mag < 1e-12 {
        let u_perp = u_r.any_perpendicular();
        let u_norm = u_r.cross(&u_perp).normalized();
        return u_r * acceleration_rsw.0
            + u_perp * acceleration_rsw.1
            + u_norm * acceleration_rsw.2;
    }

    let u_h = h_vec / h_mag;
    let u_theta = u_h.cross(&u_r).normalized();

    u_r * acceleration_rsw.0 + u_theta * acceleration_rsw.1 + u_h * acceleration_rsw.2
}

pub fn gauss_variational_equations(
    elements: &OsculatingElements,
    perturbing_accel_rsw: Vector3,
    mu: GravitationalParameter,
) -> GaussVariationalRates {
    let mu_val = mu.value();
    let a = elements.semi_major_axis.value();
    let e = elements.eccentricity;
    let inc = elements.inclination.value();
    let omega = elements.argument_of_periapsis.value();
    let nu = elements.true_anomaly.value();

    if mu_val <= 0.0 || a <= 0.0 || e < 0.0 || e >= 1.0 || !a.is_finite() || !mu_val.is_finite() {
        return GaussVariationalRates::zero();
    }

    let p = a * (1.0 - e * e);
    let h = (mu_val * p).sqrt();
    let denom = 1.0 + e * nu.cos();

    if h <= 1e-12 || denom <= 1e-12 {
        return GaussVariationalRates::zero();
    }

    let r = p / denom;
    let u = omega + nu;

    let f_r = perturbing_accel_rsw.0;
    let f_theta = perturbing_accel_rsw.1;
    let f_h = perturbing_accel_rsw.2;

    let sin_nu = nu.sin();
    let cos_nu = nu.cos();
    let sin_u = u.sin();
    let cos_u = u.cos();
    let sin_inc = inc.sin();
    let cos_inc = inc.cos();

    let da_dt = (2.0 * a * a / h) * (e * sin_nu * f_r + (p / r) * f_theta);
    let de_dt = (1.0 / h) * (p * sin_nu * f_r + ((p + r) * cos_nu + r * e) * f_theta);
    let di_dt = (r * cos_u / h) * f_h;

    let draan_dt = if sin_inc.abs() > 1e-10 {
        (r * sin_u / (h * sin_inc)) * f_h
    } else {
        0.0
    };

    let domega_dt = if e > 1e-8 {
        let part1 = (1.0 / (h * e)) * (-p * cos_nu * f_r + (p + r) * sin_nu * f_theta);
        let part2 = if sin_inc.abs() > 1e-10 {
            (r * sin_u * cos_inc / (h * sin_inc)) * f_h
        } else {
            0.0
        };
        part1 - part2
    } else {
        0.0
    };

    let dnu_dt = (h / (r * r))
        + if e > 1e-8 {
            (1.0 / (h * e)) * (p * cos_nu * f_r - (p + r) * sin_nu * f_theta)
        } else {
            0.0
        };

    GaussVariationalRates::new(
        Speed::new(da_dt),
        de_dt,
        AngularVelocity::new(di_dt),
        AngularVelocity::new(draan_dt),
        AngularVelocity::new(domega_dt),
        AngularVelocity::new(dnu_dt),
    )
}

pub fn gauss_variational_rates_from_inertial_perturbation(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    perturbing_accel_inertial: AccelerationVector,
    mu: GravitationalParameter,
) -> GaussVariationalRates {
    let rsw = inertial_to_rsw_acceleration(
        position.raw(),
        velocity.raw(),
        perturbing_accel_inertial.raw(),
    );
    gauss_variational_equations(elements, rsw, mu)
}
