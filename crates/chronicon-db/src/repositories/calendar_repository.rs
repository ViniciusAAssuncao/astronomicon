use crate::error::DbResult;
use crate::models::{CalendarDefinitionRow, CalendarTrackedMoonRow};
use crate::repositories::fetch::{fetch_all, fetch_all_by_param, fetch_optional_by_param};
use chronicon_core::domain::{
    CalendarDefinition, CalendarMoonReference, CalendarStructureKind, CalendarTrackedMoon,
    DayConvention, YearConvention,
};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, planet_id, name, structure_kind, \
    epoch_seconds_since_j2000, day_convention, year_convention, \
    reference_moon_planet_id, reference_moon_minor_planet_id, \
    founding_event_description FROM calendar_definitions";

const TRACKED_MOONS_QUERY: &str = "SELECT id, calendar_id, moon_planet_id, \
    moon_minor_planet_id FROM calendar_tracked_moons";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<CalendarDefinition>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let base_row =
        fetch_optional_by_param::<CalendarDefinitionRow, _>(pool, &query, id.to_string()).await?;

    let row = match base_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let tracked_moons = list_tracked_moons(pool, id).await?;
    let def = row.to_domain(tracked_moons)?;
    Ok(Some(def))
}

pub async fn list_by_planet(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> DbResult<Vec<CalendarDefinition>> {
    let query = format!("{BASE_QUERY} WHERE planet_id = ? ORDER BY name ASC");
    let rows =
        fetch_all_by_param::<CalendarDefinitionRow, _>(pool, &query, planet_id.to_string()).await?;

    let mut calendars = Vec::with_capacity(rows.len());
    for row in rows {
        let cal_id = Uuid::parse_str(&row.id)?;
        let tracked_moons = list_tracked_moons(pool, &cal_id).await?;
        calendars.push(row.to_domain(tracked_moons)?);
    }

    Ok(calendars)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<CalendarDefinition>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    let rows = fetch_all::<CalendarDefinitionRow>(pool, &query).await?;

    let mut calendars = Vec::with_capacity(rows.len());
    for row in rows {
        let cal_id = Uuid::parse_str(&row.id)?;
        let tracked_moons = list_tracked_moons(pool, &cal_id).await?;
        calendars.push(row.to_domain(tracked_moons)?);
    }

    Ok(calendars)
}

pub async fn list_tracked_moons(
    pool: &SqlitePool,
    calendar_id: &Uuid,
) -> DbResult<Vec<CalendarTrackedMoon>> {
    let query = format!("{TRACKED_MOONS_QUERY} WHERE calendar_id = ? ORDER BY id ASC");
    let rows = fetch_all_by_param::<CalendarTrackedMoonRow, _>(
        pool,
        &query,
        calendar_id.to_string(),
    )
    .await?;

    let mut tracked = Vec::with_capacity(rows.len());
    for row in rows {
        tracked.push(CalendarTrackedMoon::try_from(row)?);
    }

    Ok(tracked)
}

pub async fn insert(pool: &SqlitePool, calendar: &CalendarDefinition) -> DbResult<()> {
    let structure_kind_str = match calendar.structure() {
        CalendarStructureKind::SolarOnly => "SolarOnly",
        CalendarStructureKind::LunarOnly => "LunarOnly",
        CalendarStructureKind::Lunisolar => "Lunisolar",
    };

    let (ref_planet, ref_minor) = match calendar.reference_moon() {
        Some(CalendarMoonReference::Planet(id)) => (Some(id.to_string()), None),
        Some(CalendarMoonReference::MinorPlanet(id)) => (None, Some(id.to_string())),
        None => (None, None),
    };

    let day_conv_str = match calendar.day_convention() {
        DayConvention::Solar => "Solar",
        DayConvention::Sidereal => "Sidereal",
    };

    let year_conv_str = match calendar.year_convention() {
        YearConvention::Sidereal => "Sidereal",
        YearConvention::Tropical => "Tropical",
        YearConvention::Anomalistic => "Anomalistic",
    };

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO calendar_definitions (
            id, planet_id, name, structure_kind, epoch_seconds_since_j2000,
            day_convention, year_convention, reference_moon_planet_id,
            reference_moon_minor_planet_id, founding_event_description
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(calendar.id().to_string())
    .bind(calendar.planet_id().to_string())
    .bind(calendar.name())
    .bind(structure_kind_str)
    .bind(calendar.epoch().value())
    .bind(day_conv_str)
    .bind(year_conv_str)
    .bind(ref_planet)
    .bind(ref_minor)
    .bind(calendar.founding_event_description())
    .execute(&mut *tx)
    .await?;

    for tracked in calendar.tracked_moons() {
        let (moon_planet, moon_minor) = match tracked.moon() {
            CalendarMoonReference::Planet(id) => (Some(id.to_string()), None),
            CalendarMoonReference::MinorPlanet(id) => (None, Some(id.to_string())),
        };

        sqlx::query(
            "INSERT INTO calendar_tracked_moons (
                id, calendar_id, moon_planet_id, moon_minor_planet_id
            ) VALUES (?, ?, ?, ?)",
        )
        .bind(tracked.id().to_string())
        .bind(tracked.calendar_id().to_string())
        .bind(moon_planet)
        .bind(moon_minor)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &Uuid) -> DbResult<bool> {
    let res = sqlx::query("DELETE FROM calendar_definitions WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
