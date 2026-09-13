use crate::math::intercalation::{
    all_candidate_intercalation_cycles, derive_refined_intercalation_rule, RefinedIntercalationRule,
    SingleIntercalationRule,
};
use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntercalationAnalysis {
    pub day_duration: Duration,
    pub year_duration: Duration,
    pub exact_days_per_year: f64,
    pub primary_rule: SingleIntercalationRule,
    pub refined_rule: RefinedIntercalationRule,
    pub candidate_cycles: Vec<SingleIntercalationRule>,
}

impl IntercalationAnalysis {
    pub fn new(
        day_duration: Duration,
        year_duration: Duration,
        exact_days_per_year: f64,
        primary_rule: SingleIntercalationRule,
        refined_rule: RefinedIntercalationRule,
        candidate_cycles: Vec<SingleIntercalationRule>,
    ) -> Self {
        Self {
            day_duration,
            year_duration,
            exact_days_per_year,
            primary_rule,
            refined_rule,
            candidate_cycles,
        }
    }

    pub fn day_duration(&self) -> Duration {
        self.day_duration
    }

    pub fn year_duration(&self) -> Duration {
        self.year_duration
    }

    pub fn exact_days_per_year(&self) -> f64 {
        self.exact_days_per_year
    }

    pub fn primary_rule(&self) -> &SingleIntercalationRule {
        &self.primary_rule
    }

    pub fn refined_rule(&self) -> &RefinedIntercalationRule {
        &self.refined_rule
    }

    pub fn candidate_cycles(&self) -> &[SingleIntercalationRule] {
        &self.candidate_cycles
    }

    pub fn drift_days_per_millennium(&self) -> f64 {
        self.refined_rule.drift_days_per_millennium
    }

    pub fn years_per_day_drift(&self) -> Option<f64> {
        self.refined_rule.years_per_day_drift
    }
}

pub fn analyze_intercalation(
    day_duration: Duration,
    year_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<IntercalationAnalysis> {
    let t_day = day_duration.value();
    let t_year = year_duration.value();

    if !t_day.is_finite() || !t_year.is_finite() || t_day <= 0.0 || t_year <= 0.0 {
        return None;
    }

    let exact_days = t_year / t_day;
    if !exact_days.is_finite() || exact_days <= 0.0 {
        return None;
    }

    let refined_rule = derive_refined_intercalation_rule(
        day_duration,
        year_duration,
        max_primary_denominator,
        max_secondary_denominator,
    )?;

    let candidate_cycles =
        all_candidate_intercalation_cycles(day_duration, year_duration, max_secondary_denominator);

    Some(IntercalationAnalysis::new(
        day_duration,
        year_duration,
        exact_days,
        refined_rule.primary_rule,
        refined_rule,
        candidate_cycles,
    ))
}