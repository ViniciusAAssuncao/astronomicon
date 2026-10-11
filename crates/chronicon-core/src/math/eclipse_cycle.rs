use crate::constants::constants::{
    DEFAULT_MAX_SAROS_SEARCH_MONTHS, DEFAULT_SAROS_ANOMALISTIC_TOLERANCE,
    DEFAULT_SAROS_DRACONIC_TOLERANCE,
};
use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SarosLikeCycle {
    pub synodic_month_count: u32,
    pub draconic_month_count: u32,
    pub anomalistic_month_count: u32,
    pub duration: Duration,
    pub synodic_draconic_discrepancy: Duration,
    pub synodic_anomalistic_discrepancy: Duration,
}

impl SarosLikeCycle {
    pub fn new(
        synodic_month_count: u32,
        draconic_month_count: u32,
        anomalistic_month_count: u32,
        duration: Duration,
        synodic_draconic_discrepancy: Duration,
        synodic_anomalistic_discrepancy: Duration,
    ) -> Self {
        Self {
            synodic_month_count,
            draconic_month_count,
            anomalistic_month_count,
            duration,
            synodic_draconic_discrepancy,
            synodic_anomalistic_discrepancy,
        }
    }
}

pub fn find_saros_like_cycle(
    synodic_month: Duration,
    draconic_month: Duration,
    anomalistic_month: Duration,
    max_search_months: Option<u32>,
    draconic_tolerance: Option<f64>,
    anomalistic_tolerance: Option<f64>,
) -> Option<SarosLikeCycle> {
    let t_syn = synodic_month.value();
    let t_drac = draconic_month.value();
    let t_anom = anomalistic_month.value();

    if !t_syn.is_finite()
        || !t_drac.is_finite()
        || !t_anom.is_finite()
        || t_syn <= 0.0
        || t_drac <= 0.0
        || t_anom <= 0.0
    {
        return None;
    }

    let r_d = t_syn / t_drac;
    let r_a = t_syn / t_anom;

    if (r_d - 1.0).abs() < 1e-12 && (r_a - 1.0).abs() < 1e-12 {
        return Some(SarosLikeCycle::new(
            1,
            1,
            1,
            synodic_month,
            Duration::new(0.0),
            Duration::new(0.0),
        ));
    }

    let max_s = max_search_months.unwrap_or(DEFAULT_MAX_SAROS_SEARCH_MONTHS);
    let tol_d = draconic_tolerance.unwrap_or(DEFAULT_SAROS_DRACONIC_TOLERANCE);
    let tol_a = anomalistic_tolerance.unwrap_or(DEFAULT_SAROS_ANOMALISTIC_TOLERANCE);

    let mut best_cycle: Option<(u32, u32, u32, f64, f64, f64)> = None;

    for s in 1..=max_s {
        let s_f = s as f64;
        let d_f = (s_f * r_d).round();
        let a_f = (s_f * r_a).round();

        let d = d_f as u32;
        let a = a_f as u32;

        if d == 0 || a == 0 {
            continue;
        }

        let t_s = s_f * t_syn;
        let t_d_tot = d_f * t_drac;
        let t_a_tot = a_f * t_anom;

        let diff_d = (t_s - t_d_tot).abs();
        let diff_a = (t_s - t_a_tot).abs();

        let err_d = diff_d / t_drac;
        let err_a = diff_a / t_anom;

        if err_d <= tol_d && err_a <= tol_a {
            let score = err_d + 0.5 * err_a;
            match best_cycle {
                None => {
                    best_cycle = Some((s, d, a, diff_d, diff_a, score));
                }
                Some((_, _, _, _, _, best_score)) => {
                    if score < best_score {
                        best_cycle = Some((s, d, a, diff_d, diff_a, score));
                    }
                }
            }
        }
    }

    best_cycle.map(|(s, d, a, diff_d, diff_a, _)| {
        SarosLikeCycle::new(
            s,
            d,
            a,
            Duration::new((s as f64) * t_syn),
            Duration::new(diff_d),
            Duration::new(diff_a),
        )
    })
}
