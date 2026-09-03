use crate::constants::{ECCENTRICITY_CIRCULAR_TOLERANCE, ECCENTRICITY_PARABOLIC_TOLERANCE};
use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::hyperbolic::{
    hyperbolic_mean_anomaly_from_true_anomaly, hyperbolic_mean_motion,
};
use crate::math::orbital::parabolic::parabolic_time_since_periapsis;
use crate::math::orbital::{OrbitType, OsculatingElements};
use astronomicon_core::math::kepler::mean_motion;
use astronomicon_core::units::{Angle, Duration, GravitationalParameter, Length};
use std::f64::consts::TAU;

pub fn semi_latus_rectum(elements: &OsculatingElements) -> f64 {
    let e = elements.eccentricity();
    let a = elements.semi_major_axis().value();
    if elements.orbit_type() == OrbitType::Parabolic
        || (1.0 - e).abs() < ECCENTRICITY_PARABOLIC_TOLERANCE
    {
        2.0 * elements.periapsis_distance().value()
    } else if e < 1.0 {
        a * (1.0 - e * e)
    } else {
        (-a) * (e * e - 1.0)
    }
}

pub fn true_anomaly_at_radius(
    elements: &OsculatingElements,
    radius: Length,
) -> Option<(Angle, Angle)> {
    let r = radius.value();
    let e = elements.eccentricity();
    let p = semi_latus_rectum(elements);

    if r <= 0.0 || p <= 0.0 || !r.is_finite() || !p.is_finite() {
        return None;
    }

    if e < ECCENTRICITY_CIRCULAR_TOLERANCE {
        return None;
    }

    let cos_nu = (p / r - 1.0) / e;
    if cos_nu < -1.0 || cos_nu > 1.0 {
        return None;
    }

    let nu_pos = cos_nu.acos();
    Some((Angle::new(nu_pos), Angle::new(TAU - nu_pos)))
}

pub fn time_from_periapsis_to_true_anomaly(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    true_anomaly: Angle,
) -> RocketDomainResult<Duration> {
    let e = elements.eccentricity();
    let nu = true_anomaly.value();

    if e < 1.0 {
        let n = mean_motion(elements.semi_major_axis(), mu).value();
        if n <= 0.0 || !n.is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "mean_motion".to_string(),
                reason: "mean motion must be positive and finite".to_string(),
            });
        }
        let cos_e = (e + nu.cos()) / (1.0 + e * nu.cos());
        let sin_e = ((1.0 - e * e).sqrt() * nu.sin()) / (1.0 + e * nu.cos());
        let e_anom = sin_e.atan2(cos_e).rem_euclid(TAU);
        let m = (e_anom - e * e_anom.sin()).rem_euclid(TAU);
        Ok(Duration::new(m / n))
    } else if (1.0 - e).abs() < ECCENTRICITY_PARABOLIC_TOLERANCE {
        Ok(parabolic_time_since_periapsis(
            elements.periapsis_distance(),
            mu,
            true_anomaly,
        ))
    } else {
        let n_h = hyperbolic_mean_motion(elements.semi_major_axis(), mu).value();
        if n_h <= 0.0 || !n_h.is_finite() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "hyperbolic_mean_motion".to_string(),
                reason: "hyperbolic mean motion must be positive and finite".to_string(),
            });
        }
        let m_h = hyperbolic_mean_anomaly_from_true_anomaly(true_anomaly, e)?;
        Ok(Duration::new(m_h / n_h))
    }
}

pub fn time_between_true_anomalies(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    nu_from: Angle,
    nu_to: Angle,
) -> RocketDomainResult<Duration> {
    let t_from = time_from_periapsis_to_true_anomaly(elements, mu, nu_from)?;
    let t_to = time_from_periapsis_to_true_anomaly(elements, mu, nu_to)?;

    if elements.eccentricity() < 1.0 {
        let period = TAU / mean_motion(elements.semi_major_axis(), mu).value();
        let mut dt = t_to.value() - t_from.value();
        if dt < 0.0 {
            dt += period;
        }
        Ok(Duration::new(dt))
    } else {
        let dt = t_to.value() - t_from.value();
        if dt < 0.0 {
            Err(RocketDomainError::InvalidInvariant {
                field: "true_anomaly_interval".to_string(),
                reason: "target true anomaly is in the past for escape trajectory".to_string(),
            })
        } else {
            Ok(Duration::new(dt))
        }
    }
}