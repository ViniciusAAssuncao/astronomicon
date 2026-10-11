use crate::definition::ResolvedCalendar;
use crate::tick::leap_rules::{
    cumulative_units_to_container, is_container_leap, units_in_container,
};
use crate::tick::lunar_resolution::resolve_lunar_tick;
use chronicon_core::math::RefinedIntercalationRule;

pub struct LunisolarTickResult {
    pub year_index: i64,
    pub day_in_year: f64,
    pub day_index: i64,
    pub month_in_year: Option<u32>,
    pub is_leap_year: bool,
    pub is_intercalation_boundary: bool,
    pub is_leap_month_boundary: Option<bool>,
}

fn months_in_year(year_index: i64, rule_miy: &RefinedIntercalationRule) -> u32 {
    units_in_container(year_index, rule_miy)
}

fn is_leap_year(year_index: i64, rule_miy: &RefinedIntercalationRule) -> bool {
    is_container_leap(year_index, rule_miy)
}

fn cumulative_months_to_year(year_index: i64, rule_miy: &RefinedIntercalationRule) -> i64 {
    cumulative_units_to_container(year_index, rule_miy)
}

fn days_in_month(abs_month_index: i64, rule_dim: &RefinedIntercalationRule) -> u32 {
    units_in_container(abs_month_index, rule_dim)
}

fn is_leap_day_in_month(abs_month_index: i64, rule_dim: &RefinedIntercalationRule) -> bool {
    is_container_leap(abs_month_index, rule_dim)
}

fn cumulative_days_to_month(
    abs_month_index: i64,
    rule_dim: &RefinedIntercalationRule,
) -> i64 {
    cumulative_units_to_container(abs_month_index, rule_dim)
}

pub fn resolve_lunisolar_tick(
    resolved: &ResolvedCalendar,
    instant_seconds: f64,
) -> LunisolarTickResult {
    let rule_miy_opt = resolved
        .intercalation_month_in_year
        .as_ref()
        .map(|i| &i.refined_rule);
    let rule_dim_opt = resolved
        .intercalation_day_in_month
        .as_ref()
        .map(|i| &i.refined_rule);

    let (rule_miy, rule_dim) = match (rule_miy_opt, rule_dim_opt) {
        (Some(m), Some(d)) => (m, d),
        _ => {
            let lunar = resolve_lunar_tick(resolved, instant_seconds);
            return LunisolarTickResult {
                year_index: lunar.year_index,
                day_in_year: lunar.day_in_year,
                day_index: lunar.day_index,
                month_in_year: lunar.month_in_year,
                is_leap_year: lunar.is_leap_year,
                is_intercalation_boundary: lunar.is_intercalation_boundary,
                is_leap_month_boundary: lunar.is_leap_month_boundary,
            };
        }
    };

    let t_day = resolved.day_duration.value();
    let total_days = instant_seconds / t_day;

    let mean_months_per_year = rule_miy.mean_units_per_container;
    let mean_days_per_month = rule_dim.mean_units_per_container;
    let mean_days_per_year = (mean_months_per_year * mean_days_per_month).max(1.0);

    let mut y = (total_days / mean_days_per_year).floor() as i64;

    let mut start_month_y = cumulative_months_to_year(y, rule_miy);
    let mut start_day_y = cumulative_days_to_month(start_month_y, rule_dim) as f64;
    let mut m_count = months_in_year(y, rule_miy) as i64;
    let mut next_start_day_y =
        cumulative_days_to_month(start_month_y + m_count, rule_dim) as f64;
    let mut year_len = next_start_day_y - start_day_y;

    while total_days < start_day_y {
        y -= 1;
        start_month_y = cumulative_months_to_year(y, rule_miy);
        start_day_y = cumulative_days_to_month(start_month_y, rule_dim) as f64;
        m_count = months_in_year(y, rule_miy) as i64;
        next_start_day_y =
            cumulative_days_to_month(start_month_y + m_count, rule_dim) as f64;
        year_len = next_start_day_y - start_day_y;
    }

    while total_days >= start_day_y + year_len {
        y += 1;
        start_month_y = cumulative_months_to_year(y, rule_miy);
        start_day_y = cumulative_days_to_month(start_month_y, rule_dim) as f64;
        m_count = months_in_year(y, rule_miy) as i64;
        next_start_day_y =
            cumulative_days_to_month(start_month_y + m_count, rule_dim) as f64;
        year_len = next_start_day_y - start_day_y;
    }

    let approx_m_in_y =
        (((total_days - start_day_y) / mean_days_per_month).floor() as i64).clamp(0, m_count - 1);
    let mut m_abs = start_month_y + approx_m_in_y;
    let mut start_day_m = cumulative_days_to_month(m_abs, rule_dim) as f64;
    let mut month_len = days_in_month(m_abs, rule_dim) as f64;

    while total_days < start_day_m {
        m_abs -= 1;
        start_day_m = cumulative_days_to_month(m_abs, rule_dim) as f64;
        month_len = days_in_month(m_abs, rule_dim) as f64;
    }

    while total_days >= start_day_m + month_len {
        m_abs += 1;
        start_day_m = cumulative_days_to_month(m_abs, rule_dim) as f64;
        month_len = days_in_month(m_abs, rule_dim) as f64;
    }

    let month_in_year = (m_abs - start_month_y) as u32;
    let day_in_month = total_days - start_day_m;
    let day_in_month_idx = day_in_month.floor() as i64;
    let day_in_year = total_days - start_day_y;
    let day_index = day_in_year.floor() as i64;

    let is_leap_yr = is_leap_year(y, rule_miy);
    let is_leap_month = is_leap_yr && (month_in_year >= rule_miy.base_units_per_container);
    let is_day_boundary = is_leap_day_in_month(m_abs, rule_dim)
        && (day_in_month_idx >= rule_dim.base_units_per_container as i64);

    LunisolarTickResult {
        year_index: y,
        day_in_year,
        day_index,
        month_in_year: Some(month_in_year),
        is_leap_year: is_leap_yr,
        is_intercalation_boundary: is_day_boundary,
        is_leap_month_boundary: Some(is_leap_month),
    }
}
