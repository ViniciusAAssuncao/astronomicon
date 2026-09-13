use crate::error::{AppError, AppResult};
use crate::skeleton::{resolve_calendar_skeleton, CalendarSkeleton};
use astronomicon_core::units::Duration;
use chronicon_core::domain::{
    analyze_intercalation, CalendarDefinition, IntercalationAnalysis, MoonMonthInfo,
};
use chronicon_db::repositories::calendar as calendar_repo;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedCalendar {
    pub definition: CalendarDefinition,
    pub skeleton: CalendarSkeleton,
    pub day_duration: Duration,
    pub year_duration: Duration,
    pub intercalation: Option<IntercalationAnalysis>,
    pub reference_moon: Option<MoonMonthInfo>,
    pub tracked_moons: Vec<MoonMonthInfo>,
}

impl ResolvedCalendar {
    pub fn definition(&self) -> &CalendarDefinition {
        &self.definition
    }

    pub fn skeleton(&self) -> &CalendarSkeleton {
        &self.skeleton
    }

    pub fn day_duration(&self) -> Duration {
        self.day_duration
    }

    pub fn year_duration(&self) -> Duration {
        self.year_duration
    }

    pub fn intercalation(&self) -> Option<&IntercalationAnalysis> {
        self.intercalation.as_ref()
    }

    pub fn reference_moon(&self) -> Option<&MoonMonthInfo> {
        self.reference_moon.as_ref()
    }

    pub fn tracked_moons(&self) -> &[MoonMonthInfo] {
        &self.tracked_moons
    }
}

pub async fn resolve_calendar_definition(
    pool: &SqlitePool,
    calendar_id: &Uuid,
) -> AppResult<ResolvedCalendar> {
    let definition = calendar_repo::get_by_id(pool, calendar_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "CalendarDefinition".to_string(),
            id: calendar_id.to_string(),
        })?;

    let skeleton = resolve_calendar_skeleton(pool, &definition.planet_id()).await?;

    let day_duration = skeleton
        .day_duration(definition.day_convention())
        .or_else(|| skeleton.day_info.sidereal_day())
        .or_else(|| skeleton.day_info.solar_day())
        .ok_or_else(|| {
            AppError::Domain(
                "Unable to resolve rotational day duration for specified calendar convention"
                    .to_string(),
            )
        })?;

    let year_duration = skeleton
        .year_duration(definition.year_convention())
        .or_else(|| skeleton.year_info.sidereal_year())
        .ok_or_else(|| {
            AppError::Domain(
                "Unable to resolve orbital year duration for specified calendar convention"
                    .to_string(),
            )
        })?;

    let intercalation = skeleton
        .intercalation_analysis(definition.day_convention(), definition.year_convention())
        .cloned()
        .or_else(|| analyze_intercalation(day_duration, year_duration, None, None));

    let reference_moon = definition.reference_moon().and_then(|ref_moon| {
        skeleton
            .moon_system
            .moons()
            .iter()
            .find(|m| m.moon_id() == ref_moon.id())
            .cloned()
    });

    let mut tracked_moons = Vec::new();
    for tracked in definition.tracked_moons() {
        if let Some(m) = skeleton
            .moon_system
            .moons()
            .iter()
            .find(|m| m.moon_id() == tracked.moon().id())
        {
            tracked_moons.push(m.clone());
        }
    }

    Ok(ResolvedCalendar {
        definition,
        skeleton,
        day_duration,
        year_duration,
        intercalation,
        reference_moon,
        tracked_moons,
    })
}
