use crate::constants::constants::{
    ASEASONAL_ECCENTRICITY_THRESHOLD, ASEASONAL_OBLIQUITY_THRESHOLD_RAD,
    DISTANCE_DRIVEN_ECCENTRICITY_THRESHOLD,
};
use astronomicon_core::units::{Angle, Duration};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

pub fn eccentric_anomaly_from_true(true_anomaly: Angle, eccentricity: f64) -> Angle {
    let nu = true_anomaly.value();
    let e = eccentricity.clamp(0.0, 0.9999999);
    let cos_nu = nu.cos();
    let sin_nu = nu.sin();
    let denom = 1.0 + e * cos_nu;

    if denom.abs() < 1e-12 {
        return Angle::new(0.0);
    }

    let cos_e = (e + cos_nu) / denom;
    let sin_e = ((1.0 - e * e).max(0.0).sqrt() * sin_nu) / denom;

    Angle::new(sin_e.atan2(cos_e).rem_euclid(TAU))
}

pub fn mean_anomaly_from_eccentric(eccentric_anomaly: Angle, eccentricity: f64) -> Angle {
    let e_anom = eccentric_anomaly.value();
    let e = eccentricity.clamp(0.0, 0.9999999);
    let m = e_anom - e * e_anom.sin();
    Angle::new(m.rem_euclid(TAU))
}

pub fn mean_anomaly_from_true(true_anomaly: Angle, eccentricity: f64) -> Angle {
    let e_anom = eccentric_anomaly_from_true(true_anomaly, eccentricity);
    mean_anomaly_from_eccentric(e_anom, eccentricity)
}

pub fn time_from_periapsis(mean_anomaly: Angle, orbital_period: Duration) -> Duration {
    let m = mean_anomaly.value().rem_euclid(TAU);
    let t_orb = orbital_period.value();

    if t_orb <= 0.0 || !t_orb.is_finite() {
        return Duration::new(0.0);
    }

    Duration::new((m / TAU) * t_orb)
}

pub fn orbital_duration_between_true_anomalies(
    nu_start: Angle,
    nu_end: Angle,
    eccentricity: f64,
    orbital_period: Duration,
) -> Duration {
    let m_start = mean_anomaly_from_true(nu_start, eccentricity).value().rem_euclid(TAU);
    let m_end = mean_anomaly_from_true(nu_end, eccentricity).value().rem_euclid(TAU);
    let delta_m = (m_end - m_start).rem_euclid(TAU);
    let delta_m_pos = if delta_m <= 0.0 { TAU } else { delta_m };

    Duration::new((delta_m_pos / TAU) * orbital_period.value())
}

pub fn cardinal_true_anomalies(solstice_true_anomaly: Angle) -> (Angle, Angle, Angle, Angle) {
    let nu_ref = solstice_true_anomaly.value();

    let spring_eq = Angle::new(nu_ref.rem_euclid(TAU));
    let summer_sol = Angle::new((nu_ref + FRAC_PI_2).rem_euclid(TAU));
    let autumn_eq = Angle::new((nu_ref + PI).rem_euclid(TAU));
    let winter_sol = Angle::new((nu_ref + 3.0 * FRAC_PI_2).rem_euclid(TAU));

    (spring_eq, summer_sol, autumn_eq, winter_sol)
}

pub fn climatic_precession_index(
    eccentricity: f64,
    longitude_of_periapsis: Angle,
    solstice_true_anomaly: Angle,
) -> f64 {
    let e = eccentricity.clamp(0.0, 1.0);
    let delta = solstice_true_anomaly.value() - longitude_of_periapsis.value();
    e * delta.sin()
}

pub fn is_aseasonal(obliquity: Option<Angle>, eccentricity: f64) -> bool {
    let obl = obliquity.map(|a| a.value().abs()).unwrap_or(0.0);
    let e = eccentricity.abs();
    obl < ASEASONAL_OBLIQUITY_THRESHOLD_RAD && e < ASEASONAL_ECCENTRICITY_THRESHOLD
}

pub fn is_distance_driven(obliquity: Option<Angle>, eccentricity: f64) -> bool {
    let obl = obliquity.map(|a| a.value().abs()).unwrap_or(0.0);
    let e = eccentricity.abs();
    obl < ASEASONAL_OBLIQUITY_THRESHOLD_RAD && e >= DISTANCE_DRIVEN_ECCENTRICITY_THRESHOLD
}

pub fn is_obliquity_dominated(obliquity: Option<Angle>, eccentricity: f64) -> bool {
    let obl = obliquity.map(|a| a.value().abs()).unwrap_or(0.0);
    let e = eccentricity.abs();
    obl >= ASEASONAL_OBLIQUITY_THRESHOLD_RAD && e < ASEASONAL_ECCENTRICITY_THRESHOLD
}