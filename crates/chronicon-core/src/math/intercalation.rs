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
    AddLeapUnits,
    SubtractLeapUnits,
}

impl IntercalationAdjustmentDirection {
    pub fn is_addition(&self) -> bool {
        matches!(self, Self::AddLeapUnits)
    }

    pub fn is_subtraction(&self) -> bool {
        matches!(self, Self::SubtractLeapUnits)
    }

    pub fn sign(&self) -> f64 {
        match self {
            Self::AddLeapUnits => 1.0,
            Self::SubtractLeapUnits => -1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SingleIntercalationRule {
    pub base_units_per_container: u32,
    pub leap_units: u32,
    pub cycle_containers: u32,
    pub mean_units_per_container: f64,
    pub container_drift: Duration,
    pub drift_units_per_thousand_containers: f64,
    pub containers_per_unit_drift: Option<f64>,
}

impl SingleIntercalationRule {
    pub fn new(
        base_units_per_container: u32,
        leap_units: u32,
        cycle_containers: u32,
        exact_units_per_container: f64,
        unit_duration: Duration,
    ) -> Self {
        let cycle_containers_safe = cycle_containers.max(1);
        let mean_units_per_container =
            base_units_per_container as f64 + (leap_units as f64 / cycle_containers_safe as f64);
        let drift_units = mean_units_per_container - exact_units_per_container;
        let container_drift = Duration::new(drift_units * unit_duration.value());
        let drift_units_per_thousand_containers = drift_units * MILLENNIUM_YEARS;
        let containers_per_unit_drift = if drift_units.abs() > 1e-15 {
            Some(1.0 / drift_units.abs())
        } else {
            None
        };

        Self {
            base_units_per_container,
            leap_units,
            cycle_containers: cycle_containers_safe,
            mean_units_per_container,
            container_drift,
            drift_units_per_thousand_containers,
            containers_per_unit_drift,
        }
    }

    pub fn base_units_per_container(&self) -> u32 {
        self.base_units_per_container
    }

    pub fn leap_units(&self) -> u32 {
        self.leap_units
    }

    pub fn cycle_containers(&self) -> u32 {
        self.cycle_containers
    }

    pub fn mean_units_per_container(&self) -> f64 {
        self.mean_units_per_container
    }

    pub fn container_drift(&self) -> Duration {
        self.container_drift
    }

    pub fn drift_units_per_thousand_containers(&self) -> f64 {
        self.drift_units_per_thousand_containers
    }

    pub fn containers_per_unit_drift(&self) -> Option<f64> {
        self.containers_per_unit_drift
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecondaryCorrection {
    pub leap_units: u32,
    pub cycle_containers: u32,
    pub direction: IntercalationAdjustmentDirection,
}

impl SecondaryCorrection {
    pub fn new(
        leap_units: u32,
        cycle_containers: u32,
        direction: IntercalationAdjustmentDirection,
    ) -> Self {
        Self {
            leap_units,
            cycle_containers: cycle_containers.max(1),
            direction,
        }
    }

    pub fn leap_units(&self) -> u32 {
        self.leap_units
    }

    pub fn cycle_containers(&self) -> u32 {
        self.cycle_containers
    }

    pub fn direction(&self) -> IntercalationAdjustmentDirection {
        self.direction
    }

    pub fn fractional_adjustment(&self) -> f64 {
        self.direction.sign() * (self.leap_units as f64 / self.cycle_containers as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RefinedIntercalationRule {
    pub base_units_per_container: u32,
    pub primary_rule: SingleIntercalationRule,
    pub secondary_correction: Option<SecondaryCorrection>,
    pub total_leap_units: u32,
    pub total_cycle_containers: u32,
    pub exact_units_per_container: f64,
    pub mean_units_per_container: f64,
    pub container_drift: Duration,
    pub drift_units_per_thousand_containers: f64,
    pub containers_per_unit_drift: Option<f64>,
}

impl RefinedIntercalationRule {
    pub fn new(
        base_units_per_container: u32,
        primary_rule: SingleIntercalationRule,
        secondary_correction: Option<SecondaryCorrection>,
        exact_units_per_container: f64,
        unit_duration: Duration,
    ) -> Self {
        let (total_leap_units, total_cycle_containers) = match secondary_correction {
            None => (primary_rule.leap_units, primary_rule.cycle_containers),
            Some(sec) => {
                let q1 = primary_rule.cycle_containers as u64;
                let q2 = sec.cycle_containers as u64;
                let q_lcm = lcm_u64(q1, q2).max(1);

                let p1_scaled = (q_lcm / q1) * (primary_rule.leap_units as u64);
                let p2_scaled = (q_lcm / q2) * (sec.leap_units as u64);

                let p_total = match sec.direction {
                    IntercalationAdjustmentDirection::AddLeapUnits => p1_scaled + p2_scaled,
                    IntercalationAdjustmentDirection::SubtractLeapUnits => {
                        p1_scaled.saturating_sub(p2_scaled)
                    }
                };

                let g = gcd_u64(p_total, q_lcm).max(1);
                ((p_total / g) as u32, (q_lcm / g) as u32)
            }
        };

        let mean_units_per_container =
            base_units_per_container as f64 + (total_leap_units as f64 / total_cycle_containers as f64);
        let drift_units = mean_units_per_container - exact_units_per_container;
        let container_drift = Duration::new(drift_units * unit_duration.value());
        let drift_units_per_thousand_containers = drift_units * MILLENNIUM_YEARS;
        let containers_per_unit_drift = if drift_units.abs() > 1e-15 {
            Some(1.0 / drift_units.abs())
        } else {
            None
        };

        Self {
            base_units_per_container,
            primary_rule,
            secondary_correction,
            total_leap_units,
            total_cycle_containers,
            exact_units_per_container,
            mean_units_per_container,
            container_drift,
            drift_units_per_thousand_containers,
            containers_per_unit_drift,
        }
    }

    pub fn base_units_per_container(&self) -> u32 {
        self.base_units_per_container
    }

    pub fn primary_rule(&self) -> &SingleIntercalationRule {
        &self.primary_rule
    }

    pub fn secondary_correction(&self) -> Option<&SecondaryCorrection> {
        self.secondary_correction.as_ref()
    }

    pub fn total_leap_units(&self) -> u32 {
        self.total_leap_units
    }

    pub fn total_cycle_containers(&self) -> u32 {
        self.total_cycle_containers
    }

    pub fn exact_units_per_container(&self) -> f64 {
        self.exact_units_per_container
    }

    pub fn mean_units_per_container(&self) -> f64 {
        self.mean_units_per_container
    }

    pub fn container_drift(&self) -> Duration {
        self.container_drift
    }

    pub fn drift_units_per_thousand_containers(&self) -> f64 {
        self.drift_units_per_thousand_containers
    }

    pub fn containers_per_unit_drift(&self) -> Option<f64> {
        self.containers_per_unit_drift
    }
}

pub fn units_per_container(unit_duration: Duration, container_duration: Duration) -> Option<f64> {
    let t_unit = unit_duration.value();
    let t_container = container_duration.value();

    if !t_unit.is_finite() || !t_container.is_finite() || t_unit <= 0.0 || t_container <= 0.0 {
        return None;
    }

    let r = t_container / t_unit;
    if !r.is_finite() || r <= 0.0 {
        None
    } else {
        Some(r)
    }
}

pub fn days_per_year(day_duration: Duration, year_duration: Duration) -> Option<f64> {
    units_per_container(day_duration, year_duration)
}

pub fn intercalation_drift_per_thousand_containers(
    mean_units_per_container: f64,
    exact_units_per_container: f64,
) -> f64 {
    (mean_units_per_container - exact_units_per_container) * MILLENNIUM_YEARS
}

pub fn intercalation_drift_per_millennium(
    mean_year_days: f64,
    exact_days_per_year: f64,
) -> f64 {
    intercalation_drift_per_thousand_containers(mean_year_days, exact_days_per_year)
}

pub fn containers_to_accumulate_one_unit_drift(
    mean_units_per_container: f64,
    exact_units_per_container: f64,
) -> Option<f64> {
    let diff = (mean_units_per_container - exact_units_per_container).abs();
    if diff <= 1e-15 || !diff.is_finite() {
        None
    } else {
        Some(1.0 / diff)
    }
}

pub fn years_to_accumulate_one_day_drift(
    mean_year_days: f64,
    exact_days_per_year: f64,
) -> Option<f64> {
    containers_to_accumulate_one_unit_drift(mean_year_days, exact_days_per_year)
}

pub fn derive_intercalation_cycle(
    unit_duration: Duration,
    container_duration: Duration,
    max_denominator: Option<u32>,
) -> Option<SingleIntercalationRule> {
    let r = units_per_container(unit_duration, container_duration)?;
    let base_units = r.floor() as u32;
    let frac = r - (base_units as f64);

    if frac < 1e-12 {
        return Some(SingleIntercalationRule::new(
            base_units,
            0,
            1,
            r,
            unit_duration,
        ));
    }

    let best = best_rational_approximation(frac, max_denominator)?;
    Some(SingleIntercalationRule::new(
        base_units,
        best.numerator,
        best.denominator,
        r,
        unit_duration,
    ))
}

pub fn all_candidate_intercalation_cycles(
    unit_duration: Duration,
    container_duration: Duration,
    max_denominator: Option<u32>,
) -> Vec<SingleIntercalationRule> {
    let r = match units_per_container(unit_duration, container_duration) {
        Some(val) => val,
        None => return Vec::new(),
    };

    let base_units = r.floor() as u32;
    let frac = r - (base_units as f64);

    if frac < 1e-12 {
        return vec![SingleIntercalationRule::new(
            base_units,
            0,
            1,
            r,
            unit_duration,
        )];
    }

    let convergents = continued_fraction_convergents(frac, max_denominator, None, None);

    convergents
        .into_iter()
        .map(|approx| {
            SingleIntercalationRule::new(
                base_units,
                approx.numerator,
                approx.denominator,
                r,
                unit_duration,
            )
        })
        .collect()
}

pub fn derive_refined_intercalation_rule(
    unit_duration: Duration,
    container_duration: Duration,
    max_primary_denominator: Option<u32>,
    max_secondary_denominator: Option<u32>,
) -> Option<RefinedIntercalationRule> {
    let r = units_per_container(unit_duration, container_duration)?;
    let base_units = r.floor() as u32;
    let frac = r - (base_units as f64);

    if frac < 1e-12 {
        let single = SingleIntercalationRule::new(base_units, 0, 1, r, unit_duration);
        return Some(RefinedIntercalationRule::new(
            base_units,
            single,
            None,
            r,
            unit_duration,
        ));
    }

    let p_max_den = max_primary_denominator.unwrap_or(DEFAULT_MAX_PRIMARY_DENOMINATOR);
    let s_max_den = max_secondary_denominator.unwrap_or(DEFAULT_MAX_SECONDARY_DENOMINATOR);

    let primary_approx = best_rational_approximation(frac, Some(p_max_den))?;
    let primary_rule = SingleIntercalationRule::new(
        base_units,
        primary_approx.numerator,
        primary_approx.denominator,
        r,
        unit_duration,
    );

    let residual_error = frac - primary_approx.value();

    if residual_error.abs() < 1e-6 {
        return Some(RefinedIntercalationRule::new(
            base_units,
            primary_rule,
            None,
            r,
            unit_duration,
        ));
    }

    let direction = if residual_error > 0.0 {
        IntercalationAdjustmentDirection::AddLeapUnits
    } else {
        IntercalationAdjustmentDirection::SubtractLeapUnits
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
        base_units,
        primary_rule,
        secondary_correction,
        r,
        unit_duration,
    ))
}
