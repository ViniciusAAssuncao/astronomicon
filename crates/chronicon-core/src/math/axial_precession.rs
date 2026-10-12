use astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT;
use astronomicon_core::units::{
    Angle, AngularVelocity, Duration, Length, Mass, MomentOfInertia,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxialPerturber {
    pub mass: Mass,
    pub semi_major_axis: Length,
    pub eccentricity: f64,
}

impl AxialPerturber {
    pub fn new(mass: Mass, semi_major_axis: Length, eccentricity: f64) -> Self {
        Self {
            mass,
            semi_major_axis,
            eccentricity,
        }
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn semi_major_axis(&self) -> Length {
        self.semi_major_axis
    }

    pub fn eccentricity(&self) -> f64 {
        self.eccentricity
    }
}

pub fn maccullagh_difference(
    j2: f64,
    planet_mass: Mass,
    equatorial_radius: Length,
) -> f64 {
    let m = planet_mass.value();
    let r = equatorial_radius.value();

    if j2 <= 0.0 || m <= 0.0 || r <= 0.0 || !j2.is_finite() || !m.is_finite() || !r.is_finite() {
        return 0.0;
    }

    j2 * m * r * r
}

pub fn dynamical_ellipticity(
    c_minus_a: f64,
    polar_moment_of_inertia: MomentOfInertia,
) -> f64 {
    let c = polar_moment_of_inertia.value();

    if c_minus_a <= 0.0 || c <= 0.0 || !c_minus_a.is_finite() || !c.is_finite() {
        return 0.0;
    }

    (c_minus_a / c).clamp(0.0, 1.0)
}

pub fn axial_precession_rate(
    dynamical_ellipticity: f64,
    spin_angular_velocity: AngularVelocity,
    obliquity: Angle,
    perturbers: &[AxialPerturber],
) -> AngularVelocity {
    let h_d = dynamical_ellipticity;
    let omega = spin_angular_velocity.value().abs();
    let eps = obliquity.value();

    if h_d <= 0.0 || omega <= 0.0 || !h_d.is_finite() || !omega.is_finite() || !eps.is_finite() {
        return AngularVelocity::new(0.0);
    }

    let cos_eps = eps.cos();
    if cos_eps.abs() <= 1e-12 {
        return AngularVelocity::new(0.0);
    }

    let mut perturber_sum = 0.0;
    for p in perturbers {
        let m_i = p.mass.value();
        let a_i = p.semi_major_axis.value();
        let e_i = p.eccentricity.clamp(0.0, 0.9999);

        if m_i <= 0.0 || a_i <= 0.0 || !m_i.is_finite() || !a_i.is_finite() {
            continue;
        }

        let denom_e = (1.0 - e_i * e_i).powf(1.5);
        if denom_e <= 0.0 {
            continue;
        }

        let term = (GRAVITATIONAL_CONSTANT * m_i) / (a_i.powi(3) * denom_e);
        if term.is_finite() && term > 0.0 {
            perturber_sum += term;
        }
    }

    if perturber_sum <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let rate = 1.5 * (h_d / omega) * cos_eps * perturber_sum;

    if !rate.is_finite() || rate <= 0.0 {
        AngularVelocity::new(0.0)
    } else {
        AngularVelocity::new(rate)
    }
}

pub fn axial_precession_period(precession_rate: AngularVelocity) -> Option<Duration> {
    let rate = precession_rate.value();

    if rate <= 0.0 || !rate.is_finite() {
        None
    } else {
        Some(Duration::new(TAU / rate))
    }
}