use crate::math::gravity::gravitational_parameter;
use crate::units::{Density, Duration, Length, Mass};
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