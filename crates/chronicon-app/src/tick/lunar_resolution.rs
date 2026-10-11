use crate::definition::ResolvedCalendar;
use crate::tick::leap_rules::{cumulative_units_to_container, resolve_container_and_unit};

pub struct LunarTickResult {
    pub year_index: i64,
    pub day_in_year: f64,
    pub day_index: i64,
    pub month_in_year: Option<u32>,
    pub is_leap_year: bool,
    pub is_intercalation_boundary: bool,
    pub is_leap_month_boundary: Option<bool>,
}

pub fn resolve_lunar_tick(
    resolved: &ResolvedCalendar,
    instant_seconds: f64,
) -> LunarTickResult {
    let t_day = resolved.day_duration.value();
    let t_syn = resolved
        .reference_moon
        .as_ref()
        .and_then(|m| m.synodic_month())
        .map(|d| d.value())
        .unwrap_or(0.0);

    let t_year = resolved.year_duration.value();
    let nominal_months_per_year = if t_syn > 0.0 && t_syn.is_finite() && t_year > 0.0 {
        (t_year / t_syn).round().max(1.0) as u32
    } else {
        12
    };

    if let Some(dim_rule) = resolved
        .intercalation_day_in_month
        .as_ref()
        .map(|i| &i.refined_rule)
    {
        let total_days = instant_seconds / t_day;
        let (abs_month, _day_in_month, _day_in_month_idx, _is_month_leap, is_day_boundary) =
            resolve_container_and_unit(total_days, dim_rule);

        let n_months = nominal_months_per_year as i64;
        let year_index = abs_month.div_euclid(n_months);
        let month_in_year = abs_month.rem_euclid(n_months) as u32;

        let year_start_abs_month = year_index * n_months;
        let year_start_day =
            cumulative_units_to_container(year_start_abs_month, dim_rule) as f64;
        let day_in_year = total_days - year_start_day;
        let day_index = day_in_year.floor() as i64;

        LunarTickResult {
            year_index,
            day_in_year,
            day_index,
            month_in_year: Some(month_in_year),
            is_leap_year: false,
            is_intercalation_boundary: is_day_boundary,
            is_leap_month_boundary: Some(false),
        }
    } else if t_syn > 0.0 && t_syn.is_finite() {
        let total_months = instant_seconds / t_syn;
        let abs_month = total_months.floor() as i64;

        let n_months = nominal_months_per_year as i64;
        let year_index = abs_month.div_euclid(n_months);
        let month_in_year = abs_month.rem_euclid(n_months) as u32;

        let year_start_abs_month = year_index * n_months;
        let year_start_seconds = (year_start_abs_month as f64) * t_syn;
        let day_in_year = (instant_seconds - year_start_seconds) / t_day;
        let day_index = day_in_year.floor() as i64;

        LunarTickResult {
            year_index,
            day_in_year,
            day_index,
            month_in_year: Some(month_in_year),
            is_leap_year: false,
            is_intercalation_boundary: false,
            is_leap_month_boundary: Some(false),
        }
    } else {
        let y = (instant_seconds / t_year).floor() as i64;
        let day_in_yr = (instant_seconds - (y as f64) * t_year) / t_day;
        let day_idx = day_in_yr.floor() as i64;

        LunarTickResult {
            year_index: y,
            day_in_year: day_in_yr,
            day_index: day_idx,
            month_in_year: None,
            is_leap_year: false,
            is_intercalation_boundary: false,
            is_leap_month_boundary: None,
        }
    }
}
