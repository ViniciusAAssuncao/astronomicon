use crate::math::intercalation::{
    all_candidate_intercalation_cycles, derive_refined_intercalation_rule, RefinedIntercalationRule,
    SingleIntercalationRule,
};
use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntercalationAnalysis {
    pub unit_duration: Duration,
    pub container_duration: Duration,
    pub exact_units_per_container: f64,
    pub primary_rule: SingleIntercalationRule,
    pub refined_rule: RefinedIntercalationRule,
    pub candidate_cycles: Vec<SingleIntercalationRule>,
}

impl IntercalationAnalysis {
    pub fn new(
        unit_duration: Duration,
        container_duration: Duration,
        exact_units_per_container: f64,
        primary_rule: SingleIntercalationRule,
        refined_rule: RefinedIntercalationRule,
        candidate_cycles: Vec<SingleIntercalationRule>,
    ) -> Self {
        Self {
            unit_duration,
            container_duration,
            exact_units_per_container,
            primary_rule,
            refined_rule,
            candidate_cycles,
        }
    }

    pub fn unit_duration(&self) -> Duration {
        self.unit_duration
    }

    pub fn container_duration(&self) -> Duration {
        self.container_duration
    }

    pub fn exact_units_per_container(&self) -> f64 {
        self.exact_units_per_container
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

    pub fn drift_units_per_thousand_containers(&self) -> f64 {
        self.refined_rule.drift_units_per_thousand_containers
    }

    pub fn containers_per_unit_drift(&self) -> Option<f64> {
        self.refined_rule.containers_per_unit_drift
    }

    pub fn drift_days_per_millennium(&self) -> f64 {
        self.drift_units_per_thousand_containers()
    }

    pub fn years_per_day_drift(&self) -> Option<f64> {
        self.containers_per_unit_drift()
    }

    pub fn day_duration(&self) -> Duration {
        self.unit_duration
    }

    pub fn year_duration(&self) -> Duration {
        self.container_duration
    }

    pub fn exact_days_per_year(&self) -> f64 {
        self.exact_units_per_container
    }
}

pub fn analyze_period_intercalation(
    unit_duration: Duration,
    container_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<IntercalationAnalysis> {
    let t_unit = unit_duration.value();
    let t_container = container_duration.value();

    if !t_unit.is_finite() || !t_container.is_finite() || t_unit <= 0.0 || t_container <= 0.0 {
        return None;
    }

    let exact_units = t_container / t_unit;
    if !exact_units.is_finite() || exact_units <= 0.0 {
        return None;
    }

    let refined_rule = derive_refined_intercalation_rule(
        unit_duration,
        container_duration,
        max_primary_denominator,
        max_secondary_denominator,
    )?;

    let candidate_cycles =
        all_candidate_intercalation_cycles(unit_duration, container_duration, max_secondary_denominator);

    Some(IntercalationAnalysis::new(
        unit_duration,
        container_duration,
        exact_units,
        refined_rule.primary_rule,
        refined_rule,
        candidate_cycles,
    ))
}

pub fn analyze_intercalation(
    day_duration: Duration,
    year_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<IntercalationAnalysis> {
    analyze_period_intercalation(
        day_duration,
        year_duration,
        max_primary_denominator,
        max_secondary_denominator,
    )
}

pub fn analyze_day_in_year_intercalation(
    day_duration: Duration,
    year_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<IntercalationAnalysis> {
    analyze_period_intercalation(
        day_duration,
        year_duration,
        max_primary_denominator,
        max_secondary_denominator,
    )
}

pub fn analyze_month_in_year_intercalation(
    synodic_month_duration: Duration,
    year_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<IntercalationAnalysis> {
    analyze_period_intercalation(
        synodic_month_duration,
        year_duration,
        max_primary_denominator,
        max_secondary_denominator,
    )
}

pub fn analyze_day_in_month_intercalation(
    day_duration: Duration,
    synodic_month_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<IntercalationAnalysis> {
    analyze_period_intercalation(
        day_duration,
        synodic_month_duration,
        max_primary_denominator,
        max_secondary_denominator,
    )
}
