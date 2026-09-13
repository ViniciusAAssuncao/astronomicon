use crate::constants::constants::{
    DEFAULT_MAX_PRIMARY_DENOMINATOR, DEFAULT_MAX_SECONDARY_DENOMINATOR, MILLENNIUM_YEARS,
};
use crate::math::rational_approximation::{
    best_rational_approximation, continued_fraction_convergents, gcd_u64, lcm_u64,
};
use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntercalationAdjustmentDirection {
    AddLeapDays,
    SubtractLeapDays,
}

impl IntercalationAdjustmentDirection {
    pub fn is_addition(&self) -> bool {
        matches!(self, Self::AddLeapDays)
    }

    pub fn is_subtraction(&self) -> bool {
        matches!(self, Self::SubtractLeapDays)
    }

    pub fn sign(&self) -> f64 {
        match self {
            Self::AddLeapDays => 1.0,
            Self::SubtractLeapDays => -1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SingleIntercalationRule {
    pub common_year_days: u32,
    pub leap_days: u32,
    pub cycle_years: u32,
    pub mean_year_days: f64,
    pub annual_drift: Duration,
    pub drift_days_per_millennium: f64,
    pub years_per_day_drift: Option<f64>,
}

impl SingleIntercalationRule {
    pub fn new(
        common_year_days: u32,
        leap_days: u32,
        cycle_years: u32,
        exact_days_per_year: f64,
        day_duration: Duration,
    ) -> Self {
        let cycle_years_safe = cycle_years.max(1);
        let mean_year_days =
            common_year_days as f64 + (leap_days as f64 / cycle_years_safe as f64);
        let annual_drift_days = mean_year_days - exact_days_per_year;
        let annual_drift = Duration::new(annual_drift_days * day_duration.value());
        let drift_days_per_millennium = annual_drift_days * MILLENNIUM_YEARS;
        let years_per_day_drift = if annual_drift_days.abs() > 1e-15 {
            Some(1.0 / annual_drift_days.abs())
        } else {
            None
        };

        Self {
            common_year_days,
            leap_days,
            cycle_years: cycle_years_safe,
            mean_year_days,
            annual_drift,
            drift_days_per_millennium,
            years_per_day_drift,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecondaryCorrection {
    pub leap_days: u32,
    pub cycle_years: u32,
    pub direction: IntercalationAdjustmentDirection,
}

impl SecondaryCorrection {
    pub fn new(
        leap_days: u32,
        cycle_years: u32,
        direction: IntercalationAdjustmentDirection,
    ) -> Self {
        Self {
            leap_days,
            cycle_years: cycle_years.max(1),
            direction,
        }
    }

    pub fn fractional_adjustment(&self) -> f64 {
        self.direction.sign() * (self.leap_days as f64 / self.cycle_years as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RefinedIntercalationRule {
    pub common_year_days: u32,
    pub primary_rule: SingleIntercalationRule,
    pub secondary_correction: Option<SecondaryCorrection>,
    pub total_leap_days: u32,
    pub total_cycle_years: u32,
    pub exact_days_per_year: f64,
    pub mean_year_days: f64,
    pub annual_drift: Duration,
    pub drift_days_per_millennium: f64,
    pub years_per_day_drift: Option<f64>,
}

impl RefinedIntercalationRule {
    pub fn new(
        common_year_days: u32,
        primary_rule: SingleIntercalationRule,
        secondary_correction: Option<SecondaryCorrection>,
        exact_days_per_year: f64,
        day_duration: Duration,
    ) -> Self {
        let (total_leap_days, total_cycle_years) = match secondary_correction {
            None => (primary_rule.leap_days, primary_rule.cycle_years),
            Some(sec) => {
                let q1 = primary_rule.cycle_years as u64;
                let q2 = sec.cycle_years as u64;
                let q_lcm = lcm_u64(q1, q2).max(1);

                let p1_scaled = (q_lcm / q1) * (primary_rule.leap_days as u64);
                let p2_scaled = (q_lcm / q2) * (sec.leap_days as u64);

                let p_total = match sec.direction {
                    IntercalationAdjustmentDirection::AddLeapDays => p1_scaled + p2_scaled,
                    IntercalationAdjustmentDirection::SubtractLeapDays => {
                        p1_scaled.saturating_sub(p2_scaled)
                    }
                };

                let g = gcd_u64(p_total, q_lcm).max(1);
                ((p_total / g) as u32, (q_lcm / g) as u32)
            }
        };

        let mean_year_days =
            common_year_days as f64 + (total_leap_days as f64 / total_cycle_years as f64);
        let annual_drift_days = mean_year_days - exact_days_per_year;
        let annual_drift = Duration::new(annual_drift_days * day_duration.value());
        let drift_days_per_millennium = annual_drift_days * MILLENNIUM_YEARS;
        let years_per_day_drift = if annual_drift_days.abs() > 1e-15 {
            Some(1.0 / annual_drift_days.abs())
        } else {
            None
        };

        Self {
            common_year_days,
            primary_rule,
            secondary_correction,
            total_leap_days,
            total_cycle_years,
            exact_days_per_year,
            mean_year_days,
            annual_drift,
            drift_days_per_millennium,
            years_per_day_drift,
        }
    }
}

pub fn days_per_year(day_duration: Duration, year_duration: Duration) -> Option<f64> {
    let t_day = day_duration.value();
    let t_year = year_duration.value();

    if !t_day.is_finite() || !t_year.is_finite() || t_day <= 0.0 || t_year <= 0.0 {
        return None;
    }

    let r = t_year / t_day;
    if !r.is_finite() || r <= 0.0 {
        None
    } else {
        Some(r)
    }
}

pub fn intercalation_drift_per_millennium(
    mean_year_days: f64,
    exact_days_per_year: f64,
) -> f64 {
    (mean_year_days - exact_days_per_year) * MILLENNIUM_YEARS
}

pub fn years_to_accumulate_one_day_drift(
    mean_year_days: f64,
    exact_days_per_year: f64,
) -> Option<f64> {
    let diff = (mean_year_days - exact_days_per_year).abs();
    if diff <= 1e-15 || !diff.is_finite() {
        None
    } else {
        Some(1.0 / diff)
    }
}

pub fn derive_intercalation_cycle(
    day_duration: Duration,
    year_duration: Duration,
    max_denominator: Option<u32>,
) -> Option<SingleIntercalationRule> {
    let r = days_per_year(day_duration, year_duration)?;
    let common_days = r.floor() as u32;
    let frac = r - (common_days as f64);

    if frac < 1e-12 {
        return Some(SingleIntercalationRule::new(
            common_days,
            0,
            1,
            r,
            day_duration,
        ));
    }

    let best = best_rational_approximation(frac, max_denominator)?;
    Some(SingleIntercalationRule::new(
        common_days,
        best.numerator,
        best.denominator,
        r,
        day_duration,
    ))
}

pub fn all_candidate_intercalation_cycles(
    day_duration: Duration,
    year_duration: Duration,
    max_denominator: Option<u32>,
) -> Vec<SingleIntercalationRule> {
    let r = match days_per_year(day_duration, year_duration) {
        Some(val) => val,
        None => return Vec::new(),
    };

    let common_days = r.floor() as u32;
    let frac = r - (common_days as f64);

    if frac < 1e-12 {
        return vec![SingleIntercalationRule::new(
            common_days,
            0,
            1,
            r,
            day_duration,
        )];
    }

    let convergents = continued_fraction_convergents(frac, max_denominator, None, None);

    convergents
        .into_iter()
        .map(|approx| {
            SingleIntercalationRule::new(
                common_days,
                approx.numerator,
                approx.denominator,
                r,
                day_duration,
            )
        })
        .collect()
}

pub fn derive_refined_intercalation_rule(
    day_duration: Duration,
    year_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<RefinedIntercalationRule> {
    let r = days_per_year(day_duration, year_duration)?;
    let common_days = r.floor() as u32;
    let frac = r - (common_days as f64);

    if frac < 1e-12 {
        let single = SingleIntercalationRule::new(common_days, 0, 1, r, day_duration);
        return Some(RefinedIntercalationRule::new(
            common_days,
            single,
            None,
            r,
            day_duration,
        ));
    }

    let p_max_den = max_primary_denominator.unwrap_or(DEFAULT_MAX_PRIMARY_DENOMINATOR);
    let s_max_den = max_secondary_denominator.unwrap_or(DEFAULT_MAX_SECONDARY_DENOMINATOR);

    let primary_approx = best_rational_approximation(frac, Some(p_max_den))?;
    let primary_rule = SingleIntercalationRule::new(
        common_days,
        primary_approx.numerator,
        primary_approx.denominator,
        r,
        day_duration,
    );

    let residual_error = frac - primary_approx.value();

    if residual_error.abs() < 1e-6 {
        return Some(RefinedIntercalationRule::new(
            common_days,
            primary_rule,
            None,
            r,
            day_duration,
        ));
    }

    let direction = if residual_error > 0.0 {
        IntercalationAdjustmentDirection::AddLeapDays
    } else {
        IntercalationAdjustmentDirection::SubtractLeapDays
    };

    let target_error_mag = residual_error.abs();
    let secondary_approx = best_rational_approximation(target_error_mag, Some(s_max_den));

    let secondary_correction = secondary_approx.and_then(|approx| {
        if approx.numerator == 0 || approx.denominator <= 1 {
            None
        } else {
            Some(SecondaryCorrection::new(
                approx.numerator,
                approx.denominator,
                direction,
            ))
        }
    });

    Some(RefinedIntercalationRule::new(
        common_days,
        primary_rule,
        secondary_correction,
        r,
        day_duration,
    ))
}