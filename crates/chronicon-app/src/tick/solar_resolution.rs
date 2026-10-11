use crate::definition::ResolvedCalendar;
use crate::tick::leap_rules::resolve_container_and_unit;

pub struct SolarTickResult {
    pub year_index: i64,
    pub day_in_year: f64,
    pub day_index: i64,
    pub month_in_year: Option<u32>,
    pub is_leap_year: bool,
    pub is_intercalation_boundary: bool,
    pub is_leap_month_boundary: Option<bool>,
}

pub fn resolve_solar_tick(
    resolved: &ResolvedCalendar,
    instant_seconds: f64,
) -> SolarTickResult {
    let t_day = resolved.day_duration.value();

    if let Some(rule) = resolved
        .intercalation_day_in_year
        .as_ref()
        .map(|i| &i.refined_rule)
    {
        let total_days = instant_seconds / t_day;
        let (y, day_in_yr, day_idx, is_leap, is_boundary) =
            resolve_container_and_unit(total_days, rule);

        SolarTickResult {
            year_index: y,
            day_in_year: day_in_yr,
            day_index: day_idx,
            month_in_year: None,
            is_leap_year: is_leap,
            is_intercalation_boundary: is_boundary,
            is_leap_month_boundary: None,
        }
    } else {
        let t_year = resolved.year_duration.value();
        let y = (instant_seconds / t_year).floor() as i64;
        let day_in_yr = (instant_seconds - (y as f64) * t_year) / t_day;
        let day_idx = day_in_yr.floor() as i64;

        SolarTickResult {
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
