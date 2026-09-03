use super::anomalies::{time_between_true_anomalies, true_anomaly_at_radius};
use crate::constants::{ENCOUNTER_BISECTION_ITERATIONS, MIN_ENCOUNTER_SEARCH_STEPS};
use crate::error::RocketDomainResult;
use crate::math::orbital::sphere_of_influence::CelestialBodySoi;
use crate::math::orbital::universal::propagate_universal_state_vectors;
use crate::math::orbital::OsculatingElements;
use astronomicon_core::units::{
    Angle, Duration, GravitationalParameter, Length, Position, VelocityVector,
};
use std::f64::consts::TAU;

pub fn find_soi_exit_event(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    soi_radius: Length,
) -> RocketDomainResult<Option<(Angle, Duration)>> {
    let r_soi = soi_radius.value();
    if r_soi <= 0.0 || !r_soi.is_finite() {
        return Ok(None);
    }

    if let Some(r_apo) = elements.apoapsis_distance() {
        if r_apo.value() < r_soi {
            return Ok(None);
        }
    }

    let Some((nu1, nu2)) = true_anomaly_at_radius(elements, soi_radius) else {
        return Ok(None);
    };

    let nu_curr = elements.true_anomaly().value().rem_euclid(TAU);
    let nu_exit = if elements.eccentricity() < 1.0 {
        if nu_curr <= nu1.value() {
            nu1
        } else if nu_curr <= nu2.value() {
            nu2
        } else {
            nu1
        }
    } else {
        nu1
    };

    let dt = time_between_true_anomalies(elements, mu, elements.true_anomaly(), nu_exit)?;
    Ok(Some((nu_exit, dt)))
}

pub fn find_body_encounter(
    initial_pos: Position,
    initial_vel: VelocityVector,
    primary_mu: GravitationalParameter,
    target_body: &CelestialBodySoi,
    max_search_duration: Duration,
    steps: usize,
) -> RocketDomainResult<Option<Duration>> {
    let dt_total = max_search_duration.value();
    if dt_total <= 0.0 || steps < MIN_ENCOUNTER_SEARCH_STEPS {
        return Ok(None);
    }

    let h = dt_total / (steps as f64);
    let r_target = target_body.soi_radius().value();
    let target_r_sq = r_target * r_target;

    let dist_sq_at = |t_sec: f64| -> RocketDomainResult<f64> {
        let (r_v, _) = propagate_universal_state_vectors(
            initial_pos,
            initial_vel,
            primary_mu,
            Duration::new(t_sec),
        )?;
        let diff = r_v.raw() - target_body.position().raw();
        Ok(diff.dot(&diff))
    };

    let mut prev_t = 0.0;
    let prev_f = dist_sq_at(0.0)? - target_r_sq;

    if prev_f <= 0.0 {
        return Ok(Some(Duration::new(0.0)));
    }

    for i in 1..=steps {
        let curr_t = (i as f64) * h;
        let curr_f = dist_sq_at(curr_t)? - target_r_sq;

        if curr_f <= 0.0 {
            let mut t_low = prev_t;
            let mut t_high = curr_t;
            for _ in 0..ENCOUNTER_BISECTION_ITERATIONS {
                let t_mid = 0.5 * (t_low + t_high);
                let f_mid = dist_sq_at(t_mid)? - target_r_sq;
                if f_mid <= 0.0 {
                    t_high = t_mid;
                } else {
                    t_low = t_mid;
                }
            }
            return Ok(Some(Duration::new(0.5 * (t_low + t_high))));
        }

        prev_t = curr_t;
    }

    Ok(None)
}

pub fn find_atmospheric_entry_event(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    entry_interface_radius: Length,
) -> RocketDomainResult<Option<(Angle, Duration)>> {
    let r_entry = entry_interface_radius.value();
    if r_entry <= 0.0 || !r_entry.is_finite() {
        return Ok(None);
    }

    if elements.periapsis_distance().value() >= r_entry {
        return Ok(None);
    }

    let Some((_, nu2)) = true_anomaly_at_radius(elements, entry_interface_radius) else {
        return Ok(None);
    };

    let dt = time_between_true_anomalies(elements, mu, elements.true_anomaly(), nu2)?;
    Ok(Some((nu2, dt)))
}