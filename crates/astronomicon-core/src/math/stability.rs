use crate::units::constants::{
    MARDLING_AARSETH_CRITICAL_COEFFICIENT, MARDLING_AARSETH_INCLINATION_COEFFICIENT,
    MARDLING_AARSETH_MASS_EXPONENT,
};
use crate::units::{Angle, Length, Mass};
use std::f64::consts::PI;

pub fn hill_sphere_radius(
    semi_major_axis: Length,
    eccentricity: f64,
    body_mass: Mass,
    parent_mass: Mass,
) -> Length {
    if semi_major_axis.value() <= 0.0 || body_mass.value() <= 0.0 || parent_mass.value() <= 0.0 {
        return Length::new(0.0);
    }
    let e = eccentricity.clamp(0.0, 1.0);
    let periapsis = semi_major_axis.value() * (1.0 - e);
    let mass_ratio = body_mass.value() / (3.0 * parent_mass.value());
    Length::new(periapsis * mass_ratio.cbrt())
}

pub fn mardling_aarseth_critical_ratio(
    inner_mass: Mass,
    outer_mass: Mass,
    outer_eccentricity: f64,
    mutual_inclination: Angle,
) -> f64 {
    if inner_mass.value() <= 0.0 || outer_mass.value() <= 0.0 {
        return 0.0;
    }
    let q = outer_mass.value() / inner_mass.value();
    let e_out = outer_eccentricity.clamp(0.0, 0.9999);
    let denom = (1.0 - e_out).max(1e-6).sqrt();
    let bracket = ((1.0 + q) * (1.0 + e_out) / denom).powf(MARDLING_AARSETH_MASS_EXPONENT);
    let inc_term = 1.0 - MARDLING_AARSETH_INCLINATION_COEFFICIENT * (mutual_inclination.value().rem_euclid(PI) / PI);
    MARDLING_AARSETH_CRITICAL_COEFFICIENT * bracket * inc_term
}

pub fn mardling_aarseth_stability_ratio(
    inner_semi_major_axis: Length,
    outer_periapsis: Length,
) -> f64 {
    if inner_semi_major_axis.value() <= 0.0 {
        0.0
    } else {
        outer_periapsis.value() / inner_semi_major_axis.value()
    }
}

pub fn is_hierarchically_stable(
    inner_semi_major_axis: Length,
    outer_periapsis: Length,
    inner_mass: Mass,
    outer_mass: Mass,
    outer_eccentricity: f64,
    mutual_inclination: Angle,
) -> bool {
    let actual = mardling_aarseth_stability_ratio(inner_semi_major_axis, outer_periapsis);
    let critical = mardling_aarseth_critical_ratio(
        inner_mass,
        outer_mass,
        outer_eccentricity,
        mutual_inclination,
    );
    actual >= critical
}