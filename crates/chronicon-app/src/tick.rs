use crate::definition::resolve_calendar_definition;
use crate::error::AppResult;
use crate::moon_phase::{
    compute_eclipse_proximity, compute_moon_phase, EclipseProximityInfo, MoonPhaseInfo,
};
use crate::season_tracking::{resolve_season_state, SeasonState};
use astronomicon_core::units::Duration;
use chronicon_core::math::{IntercalationAdjustmentDirection, RefinedIntercalationRule};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarTick {
    pub instant_seconds: f64,
    pub elapsed_since_epoch: Duration,
    pub year_index: i64,
    pub day_in_year: f64,
    pub day_index: i64,
    pub is_leap_year: bool,
    pub is_intercalation_boundary: bool,
    pub season: SeasonState,
    pub moon_phases: Vec<MoonPhaseInfo>,
    pub eclipse_proximities: Vec<EclipseProximityInfo>,
}

pub fn is_leap_year_for_rule(year_index: i64, rule: &RefinedIntercalationRule) -> bool {
    let p_rule = &rule.primary_rule;
    if p_rule.cycle_containers == 0 || p_rule.leap_units == 0 {
        return false;
    }

    let q1 = p_rule.cycle_containers as i64;
    let p1 = p_rule.leap_units as i64;
    let rem1 = year_index.rem_euclid(q1);
    let mut leap = (rem1 * p1) % q1 < p1;

    if let Some(sec) = &rule.secondary_correction {
        if sec.cycle_containers > 0 {
            let q2 = sec.cycle_containers as i64;
            if year_index.rem_euclid(q2) == 0 {
                match sec.direction {
                    IntercalationAdjustmentDirection::SubtractLeapUnits => leap = false,
                    IntercalationAdjustmentDirection::AddLeapUnits => leap = true,
                }
            }
        }
    }

    leap
}

pub fn days_in_calendar_year(year_index: i64, rule: &RefinedIntercalationRule) -> u32 {
    rule.base_units_per_container + if is_leap_year_for_rule(year_index, rule) { 1 } else { 0 }
}

pub fn cumulative_days_to_year(year_index: i64, rule: &RefinedIntercalationRule) -> i64 {
    let q_tot = (rule.total_cycle_containers as i64).max(1);
    let days_per_cycle = (q_tot * (rule.base_units_per_container as i64)) + (rule.total_leap_units as i64);

    let cycle_count = year_index.div_euclid(q_tot);
    let year_in_cycle = year_index.rem_euclid(q_tot);

    let mut days = cycle_count * days_per_cycle;
    let base_year = cycle_count * q_tot;
    for y in 0..year_in_cycle {
        days += days_in_calendar_year(base_year + y, rule) as i64;
    }

    days
}

pub fn resolve_year_and_day(
    total_days: f64,
    rule: &RefinedIntercalationRule,
) -> (i64, f64, i64, bool, bool) {
    let mean_days = rule.mean_units_per_container.max(1.0);
    let mut y = (total_days / mean_days).floor() as i64;

    let mut start_day = cumulative_days_to_year(y, rule) as f64;
    let mut year_len = days_in_calendar_year(y, rule) as f64;

    while total_days < start_day {
        y -= 1;
        start_day = cumulative_days_to_year(y, rule) as f64;
        year_len = days_in_calendar_year(y, rule) as f64;
    }

    while total_days >= start_day + year_len {
        y += 1;
        start_day = cumulative_days_to_year(y, rule) as f64;
        year_len = days_in_calendar_year(y, rule) as f64;
    }

    let day_in_year = total_days - start_day;
    let day_index = day_in_year.floor() as i64;
    let is_leap = is_leap_year_for_rule(y, rule);
    let is_boundary = is_leap && (day_index >= rule.base_units_per_container as i64);

    (y, day_in_year, day_index, is_leap, is_boundary)
}

pub async fn resolve_calendar_tick(
    pool: &SqlitePool,
    calendar_id: &Uuid,
    elapsed_since_epoch: Duration,
) -> AppResult<CalendarTick> {
    let resolved = resolve_calendar_definition(pool, calendar_id).await?;
    let t = elapsed_since_epoch.value();
    let t_day = resolved.day_duration.value();

    let (year_index, day_in_year, day_index, is_leap, is_boundary) =
        if let Some(rule) = resolved
            .intercalation_day_in_year
            .as_ref()
            .map(|i| &i.refined_rule)
        {
            let total_days = t / t_day;
            resolve_year_and_day(total_days, rule)
        } else {
            let t_year = resolved.year_duration.value();
            let y = (t / t_year).floor() as i64;
            let day_in_yr = (t - (y as f64) * t_year) / t_day;
            let day_idx = day_in_yr.floor() as i64;
            (y, day_in_yr, day_idx, false, false)
        };

    let season =
        resolve_season_state(&resolved.skeleton.seasonal_structure, elapsed_since_epoch);

    let mut moon_phases = Vec::new();
    let mut eclipse_proximities = Vec::new();

    let moons_to_track = if !resolved.tracked_moons.is_empty() {
        &resolved.tracked_moons
    } else {
        resolved.skeleton.moon_system.moons()
    };

    for moon in moons_to_track {
        moon_phases.push(compute_moon_phase(moon, elapsed_since_epoch));
        eclipse_proximities.push(compute_eclipse_proximity(moon, elapsed_since_epoch));
    }

    Ok(CalendarTick {
        instant_seconds: t,
        elapsed_since_epoch,
        year_index,
        day_in_year,
        day_index,
        is_leap_year: is_leap,
        is_intercalation_boundary: is_boundary,
        season,
        moon_phases,
        eclipse_proximities,
    })
}