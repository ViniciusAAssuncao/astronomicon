use crate::units::constants::{
    MARDLING_AARSETH_CRITICAL_COEFFICIENT, MARDLING_AARSETH_INCLINATION_COEFFICIENT,
    MARDLING_AARSETH_MASS_EXPONENT,
};
use crate::units::{Angle, Duration, Length, Mass};
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
    let inc_term = 1.0
        - MARDLING_AARSETH_INCLINATION_COEFFICIENT
            * (mutual_inclination.value().rem_euclid(PI) / PI);
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

pub fn kozai_constant(inner_eccentricity: f64, mutual_inclination: Angle) -> f64 {
    let e = inner_eccentricity;
    let cos_i = mutual_inclination.value().cos();
    (1.0 - e * e) * cos_i * cos_i
}

pub fn kozai_critical_inclination() -> Angle {
    Angle::new((3.0_f64 / 5.0).sqrt().acos())
}

pub fn kozai_max_eccentricity(initial_mutual_inclination: Angle) -> f64 {
    let cos_i0 = initial_mutual_inclination.value().cos();
    let val = 1.0 - (5.0 / 3.0) * cos_i0 * cos_i0;
    if val <= 0.0 || !val.is_finite() {
        0.0
    } else {
        val.sqrt()
    }
}

pub fn kozai_oscillation_timescale(
    inner_period: Duration,
    outer_period: Duration,
    inner_total_mass: Mass,
    outer_mass: Mass,
    outer_eccentricity: f64,
) -> Duration {
    let p_in = inner_period.value();
    let p_out = outer_period.value();
    let m_in = inner_total_mass.value();
    let m_out = outer_mass.value();
    let e_out = outer_eccentricity;

    if p_in <= 0.0
        || p_out <= 0.0
        || m_in <= 0.0
        || m_out <= 0.0
        || e_out < 0.0
        || e_out >= 1.0
        || !p_in.is_finite()
        || !p_out.is_finite()
        || !m_in.is_finite()
        || !m_out.is_finite()
        || !e_out.is_finite()
    {
        return Duration::new(0.0);
    }

    let factor_e = (1.0 - e_out * e_out).powf(1.5);
    let timescale = (p_out * p_out / p_in) * (m_in / m_out) * factor_e;
    Duration::new(timescale)
}