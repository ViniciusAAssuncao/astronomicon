pub mod leap_rules;
pub mod lunar_resolution;
pub mod lunisolar_resolution;
pub mod solar_resolution;

pub use leap_rules::{
    cumulative_units_to_container, is_container_leap, resolve_container_and_unit,
    units_in_container,
};
pub use lunar_resolution::{resolve_lunar_tick, LunarTickResult};
pub use lunisolar_resolution::{resolve_lunisolar_tick, LunisolarTickResult};
pub use solar_resolution::{resolve_solar_tick, SolarTickResult};

use crate::definition::resolve_calendar_definition;
use crate::error::AppResult;
use crate::moon_phase::{
    compute_eclipse_proximity, compute_moon_phase, EclipseProximityInfo, MoonPhaseInfo,
};
use crate::season_tracking::{resolve_season_state, SeasonState};
use astronomicon_core::units::Duration;
use chronicon_core::domain::CalendarStructureKind;
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
    pub month_in_year: Option<u32>,
    pub is_leap_year: bool,
    pub is_intercalation_boundary: bool,
    pub is_leap_month_boundary: Option<bool>,
    pub season: SeasonState,
    pub moon_phases: Vec<MoonPhaseInfo>,
    pub eclipse_proximities: Vec<EclipseProximityInfo>,
}

pub async fn resolve_calendar_tick(
    pool: &SqlitePool,
    calendar_id: &Uuid,
    elapsed_since_epoch: Duration,
) -> AppResult<CalendarTick> {
    let resolved = resolve_calendar_definition(pool, calendar_id).await?;
    let t = elapsed_since_epoch.value();

    let (
        year_index,
        day_in_year,
        day_index,
        month_in_year,
        is_leap_year,
        is_intercalation_boundary,
        is_leap_month_boundary,
    ) = match resolved.definition.structure() {
        CalendarStructureKind::SolarOnly => {
            let res = resolve_solar_tick(&resolved, t);
            (
                res.year_index,
                res.day_in_year,
                res.day_index,
                res.month_in_year,
                res.is_leap_year,
                res.is_intercalation_boundary,
                res.is_leap_month_boundary,
            )
        }
        CalendarStructureKind::LunarOnly => {
            let res = resolve_lunar_tick(&resolved, t);
            (
                res.year_index,
                res.day_in_year,
                res.day_index,
                res.month_in_year,
                res.is_leap_year,
                res.is_intercalation_boundary,
                res.is_leap_month_boundary,
            )
        }
        CalendarStructureKind::Lunisolar => {
            let res = resolve_lunisolar_tick(&resolved, t);
            (
                res.year_index,
                res.day_in_year,
                res.day_index,
                res.month_in_year,
                res.is_leap_year,
                res.is_intercalation_boundary,
                res.is_leap_month_boundary,
            )
        }
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
        month_in_year,
        is_leap_year,
        is_intercalation_boundary,
        is_leap_month_boundary,
        season,
        moon_phases,
        eclipse_proximities,
    })
}
