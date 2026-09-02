use crate::math::gravity::gravitational_parameter;
use crate::units::{Density, Duration, Length, Mass, Position, Vector3};
use std::f64::consts::PI;

pub fn rotational_flattening(
    mass: Mass,
    equatorial_radius: Length,
    rotation_period: Duration,
    oblateness_j2: f64,
) -> f64 {
    let m = mass.value();
    let r_eq = equatorial_radius.value();
    let t = rotation_period.value();

    if m <= 0.0
        || !m.is_finite()
        || r_eq <= 0.0
        || !r_eq.is_finite()
        || t <= 0.0
        || !t.is_finite()
        || !oblateness_j2.is_finite()
    {
        return 0.0;
    }

    let mu = gravitational_parameter(mass).value();
    if mu <= 0.0 || !mu.is_finite() {
        return 0.0;
    }

    let omega = (2.0 * PI) / t;
    let q = (omega * omega * r_eq * r_eq * r_eq) / mu;
    if !q.is_finite() {
        return 0.0;
    }

    let f = 1.5 * oblateness_j2 + 0.5 * q;
    if !f.is_finite() {
        return 0.0;
    }

    f.clamp(0.0, 0.6)
}

pub fn polar_radius_from_flattening(equatorial_radius: Length, flattening: f64) -> Length {
    let r_eq = equatorial_radius.value();
    if r_eq <= 0.0 || !r_eq.is_finite() {
        return Length::new(0.0);
    }

    if !flattening.is_finite() || flattening < 0.0 || flattening >= 1.0 {
        return equatorial_radius;
    }

    Length::new(r_eq * (1.0 - flattening))
}

pub fn oblate_spheroid_mean_density(
    mass: Mass,
    equatorial_radius: Length,
    polar_radius: Length,
) -> Density {
    let r_eq = equatorial_radius.value();
    let r_pol = polar_radius.value();
    let m = mass.value();

    if r_eq <= 0.0
        || r_pol <= 0.0
        || m <= 0.0
        || !r_eq.is_finite()
        || !r_pol.is_finite()
        || !m.is_finite()
    {
        Density::new(0.0)
    } else {
        let volume = (4.0 / 3.0) * PI * r_eq * r_eq * r_pol;
        Density::new(m / volume)
    }
}

pub fn iugg_mean_radius(equatorial_radius: Length, polar_radius: Length) -> Length {
    let r_eq = equatorial_radius.value();
    let r_pol = polar_radius.value();

    if r_eq <= 0.0 || r_pol <= 0.0 || !r_eq.is_finite() || !r_pol.is_finite() {
        return Length::new(0.0);
    }

    Length::new((2.0 * r_eq + r_pol) / 3.0)
}

pub fn mean_radius(equatorial_radius: Length, polar_radius: Length) -> Length {
    iugg_mean_radius(equatorial_radius, polar_radius)
}

pub fn volumetric_mean_radius(equatorial_radius: Length, polar_radius: Length) -> Length {
    let r_eq = equatorial_radius.value();
    let r_pol = polar_radius.value();

    if r_eq <= 0.0 || r_pol <= 0.0 || !r_eq.is_finite() || !r_pol.is_finite() {
        return Length::new(0.0);
    }

    Length::new((r_eq * r_eq * r_pol).cbrt())
}

pub fn geodetic_altitude_and_normal(
    equatorial_radius: Length,
    polar_radius: Length,
    body_fixed_position: Position,
) -> (Length, Vector3) {
    let a = equatorial_radius.value();
    let b = polar_radius.value();
    let p_vec = body_fixed_position.raw();
    let x = p_vec.0;
    let y = p_vec.1;
    let z = p_vec.2;

    if a <= 0.0 || b <= 0.0 || !a.is_finite() || !b.is_finite() {
        return (Length::new(0.0), Vector3::new(0.0, 0.0, 1.0));
    }

    let p = (x * x + y * y).sqrt();

    if p < 1e-12 {
        let sign = if z >= 0.0 { 1.0 } else { -1.0 };
        let normal = Vector3::new(0.0, 0.0, sign);
        let altitude = z.abs() - b;
        return (Length::new(altitude), normal);
    }

    let a2 = a * a;
    let b2 = b * b;
    let e2 = (a2 - b2) / a2;

    if e2.abs() < 1e-12 {
        let r = (p * p + z * z).sqrt();
        let normal = p_vec / r;
        let altitude = r - a;
        return (Length::new(altitude), normal);
    }

    let e_prime2 = (a2 - b2) / b2;

    let mut theta = (z * a).atan2(p * b);
    let mut phi = (z + e_prime2 * b * theta.sin().powi(3))
        .atan2(p - e2 * a * theta.cos().powi(3));

    for _ in 0..2 {
        theta = (b * phi.sin()).atan2(a * phi.cos());
        phi = (z + e_prime2 * b * theta.sin().powi(3))
            .atan2(p - e2 * a * theta.cos().powi(3));
    }

    let sin_phi = phi.sin();
    let cos_phi = phi.cos();
    let n = a / (1.0 - e2 * sin_phi * sin_phi).max(1e-12).sqrt();

    let altitude = if cos_phi.abs() > sin_phi.abs() {
        p / cos_phi - n
    } else {
        z / sin_phi - n * (1.0 - e2)
    };

    let cos_lambda = x / p;
    let sin_lambda = y / p;
    let normal = Vector3::new(cos_phi * cos_lambda, cos_phi * sin_lambda, sin_phi).normalized();

    (Length::new(altitude), normal)
}