use crate::domain::OrbitalElements;
use crate::math::kepler::mean_motion;
use crate::units::constants::SPEED_OF_LIGHT;
use crate::units::{ Angle, AngularVelocity, GravitationalParameter, Length };
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SecularPrecessionRates {
    pub nodal: AngularVelocity,
    pub apsidal: AngularVelocity,
    pub mean_anomaly_correction: AngularVelocity,
}

impl SecularPrecessionRates {
    pub fn zero() -> Self {
        Self {
            nodal: AngularVelocity::new(0.0),
            apsidal: AngularVelocity::new(0.0),
            mean_anomaly_correction: AngularVelocity::new(0.0),
        }
    }

    pub fn new(
        nodal: AngularVelocity,
        apsidal: AngularVelocity,
        mean_anomaly_correction: AngularVelocity
    ) -> Self {
        Self {
            nodal,
            apsidal,
            mean_anomaly_correction,
        }
    }
}

pub fn nodal_regression_rate_j2(
    mean_motion: AngularVelocity,
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    j2: f64,
    equatorial_radius: Length
) -> AngularVelocity {
    let n = mean_motion.value();
    let a = semi_major_axis.value();
    let r_eq = equatorial_radius.value();
    let e = eccentricity;

    if
        n <= 0.0 ||
        !n.is_finite() ||
        a <= 0.0 ||
        !a.is_finite() ||
        e < 0.0 ||
        e >= 1.0 ||
        !e.is_finite() ||
        r_eq <= 0.0 ||
        !r_eq.is_finite() ||
        !j2.is_finite() ||
        !inclination.value().is_finite()
    {
        return AngularVelocity::new(0.0);
    }

    let p = a * (1.0 - e * e);
    if p <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let ratio = r_eq / p;
    let cos_i = inclination.value().cos();
    let rate = -1.5 * n * j2 * ratio * ratio * cos_i;
    AngularVelocity::new(rate)
}

pub fn apsidal_precession_rate_j2(
    mean_motion: AngularVelocity,
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    j2: f64,
    equatorial_radius: Length
) -> AngularVelocity {
    let n = mean_motion.value();
    let a = semi_major_axis.value();
    let r_eq = equatorial_radius.value();
    let e = eccentricity;

    if
        n <= 0.0 ||
        !n.is_finite() ||
        a <= 0.0 ||
        !a.is_finite() ||
        e < 0.0 ||
        e >= 1.0 ||
        !e.is_finite() ||
        r_eq <= 0.0 ||
        !r_eq.is_finite() ||
        !j2.is_finite() ||
        !inclination.value().is_finite()
    {
        return AngularVelocity::new(0.0);
    }

    let p = a * (1.0 - e * e);
    if p <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let ratio = r_eq / p;
    let cos_i = inclination.value().cos();
    let rate = 0.75 * n * j2 * ratio * ratio * (5.0 * cos_i * cos_i - 1.0);
    AngularVelocity::new(rate)
}

pub fn mean_anomaly_secular_rate_j2(
    mean_motion: AngularVelocity,
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    j2: f64,
    equatorial_radius: Length
) -> AngularVelocity {
    let n = mean_motion.value();
    let a = semi_major_axis.value();
    let r_eq = equatorial_radius.value();
    let e = eccentricity;

    if
        n <= 0.0 ||
        !n.is_finite() ||
        a <= 0.0 ||
        !a.is_finite() ||
        e < 0.0 ||
        e >= 1.0 ||
        !e.is_finite() ||
        r_eq <= 0.0 ||
        !r_eq.is_finite() ||
        !j2.is_finite() ||
        !inclination.value().is_finite()
    {
        return AngularVelocity::new(0.0);
    }

    let p = a * (1.0 - e * e);
    if p <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let ratio = r_eq / p;
    let cos_i = inclination.value().cos();
    let sqrt_factor = (1.0 - e * e).max(0.0).sqrt();
    let rate = 0.75 * n * j2 * ratio * ratio * sqrt_factor * (3.0 * cos_i * cos_i - 1.0);
    AngularVelocity::new(rate)
}

pub fn apsidal_precession_rate_relativistic(
    mean_motion: AngularVelocity,
    semi_major_axis: Length,
    eccentricity: f64,
    mu: GravitationalParameter
) -> AngularVelocity {
    let n = mean_motion.value();
    let a = semi_major_axis.value();
    let e = eccentricity;
    let mu_val = mu.value();
    let c = SPEED_OF_LIGHT;

    if
        n <= 0.0 ||
        !n.is_finite() ||
        a <= 0.0 ||
        !a.is_finite() ||
        e < 0.0 ||
        e >= 1.0 ||
        !e.is_finite() ||
        mu_val <= 0.0 ||
        !mu_val.is_finite()
    {
        return AngularVelocity::new(0.0);
    }

    let p = a * (1.0 - e * e);
    if p <= 0.0 {
        return AngularVelocity::new(0.0);
    }

    let rate = (3.0 * n * mu_val) / (c * c * p);
    AngularVelocity::new(rate)
}

pub fn resolve_secular_precession(
    elements: &OrbitalElements,
    mu: GravitationalParameter,
    parent_j2: Option<f64>,
    parent_equatorial_radius: Option<Length>
) -> SecularPrecessionRates {
    let n = mean_motion(elements.semi_major_axis(), mu);
    let apsidal_gr = apsidal_precession_rate_relativistic(
        n,
        elements.semi_major_axis(),
        elements.eccentricity(),
        mu
    );

    match (parent_j2, parent_equatorial_radius) {
        (Some(j2), Some(r_eq)) if
            j2.is_finite() &&
            r_eq.value() > 0.0 &&
            r_eq.value().is_finite()
        => {
            let nodal_j2 = nodal_regression_rate_j2(
                n,
                elements.semi_major_axis(),
                elements.eccentricity(),
                elements.inclination(),
                j2,
                r_eq
            );
            let apsidal_j2 = apsidal_precession_rate_j2(
                n,
                elements.semi_major_axis(),
                elements.eccentricity(),
                elements.inclination(),
                j2,
                r_eq
            );
            let m_corr = mean_anomaly_secular_rate_j2(
                n,
                elements.semi_major_axis(),
                elements.eccentricity(),
                elements.inclination(),
                j2,
                r_eq
            );

            SecularPrecessionRates {
                nodal: nodal_j2,
                apsidal: apsidal_j2 + apsidal_gr,
                mean_anomaly_correction: m_corr,
            }
        }
        _ =>
            SecularPrecessionRates {
                nodal: AngularVelocity::new(0.0),
                apsidal: apsidal_gr,
                mean_anomaly_correction: AngularVelocity::new(0.0),
            },
    }
}
