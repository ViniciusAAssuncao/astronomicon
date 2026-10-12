use crate::error::DbError;
use crate::models::moon_parsing::parse_required_moon_reference;
use chronicon_core::domain::CalendarTrackedMoon;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct CalendarTrackedMoonRow {
    pub id: String,
    pub calendar_id: String,
    pub moon_planet_id: Option<String>,
    pub moon_minor_planet_id: Option<String>,
}

impl TryFrom<CalendarTrackedMoonRow> for CalendarTrackedMoon {
    type Error = DbError;

    fn try_from(row: CalendarTrackedMoonRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let calendar_id = Uuid::parse_str(&row.calendar_id)?;
        let moon = parse_required_moon_reference(row.moon_planet_id, row.moon_minor_planet_id)?;
        Ok(CalendarTrackedMoon::new(id, calendar_id, moon))
    }
}