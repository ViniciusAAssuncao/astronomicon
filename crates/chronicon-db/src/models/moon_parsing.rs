use crate::error::DbError;
use chronicon_core::domain::CalendarMoonReference;
use chronicon_core::error::DomainError;
use uuid::Uuid;

pub fn parse_optional_moon_reference(
    planet_id: Option<String>,
    minor_planet_id: Option<String>,
) -> Result<Option<CalendarMoonReference>, DbError> {
    match (planet_id, minor_planet_id) {
        (None, None) => Ok(None),
        (Some(id), None) => Ok(Some(CalendarMoonReference::Planet(Uuid::parse_str(&id)?))),
        (None, Some(id)) => Ok(Some(CalendarMoonReference::MinorPlanet(Uuid::parse_str(&id)?))),
        (Some(_), Some(_)) => Err(DbError::Domain(DomainError::InvalidInvariant {
            field: "reference_moon".to_string(),
            reason: "multiple moon references specified".to_string(),
        })),
    }
}

pub fn parse_required_moon_reference(
    planet_id: Option<String>,
    minor_planet_id: Option<String>,
) -> Result<CalendarMoonReference, DbError> {
    match (planet_id, minor_planet_id) {
        (Some(id), None) => Ok(CalendarMoonReference::Planet(Uuid::parse_str(&id)?)),
        (None, Some(id)) => Ok(CalendarMoonReference::MinorPlanet(Uuid::parse_str(&id)?)),
        _ => Err(DbError::Domain(DomainError::InvalidInvariant {
            field: "tracked_moon".to_string(),
            reason: "exactly one moon reference must be specified".to_string(),
        })),
    }
}