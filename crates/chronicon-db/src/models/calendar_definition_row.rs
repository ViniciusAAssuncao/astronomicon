use crate::error::DbError;
use crate::models::moon_parsing::parse_optional_moon_reference;
use astronomicon_core::units::Duration;
use chronicon_core::domain::{
    CalendarDefinition, CalendarStructureKind, CalendarTrackedMoon, DayConvention,
    YearConvention,
};
use chronicon_core::error::DomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct CalendarDefinitionRow {
    pub id: String,
    pub planet_id: String,
    pub name: String,
    pub structure_kind: String,
    pub epoch_seconds_since_j2000: f64,
    pub day_convention: String,
    pub year_convention: String,
    pub reference_moon_planet_id: Option<String>,
    pub reference_moon_minor_planet_id: Option<String>,
    pub founding_event_description: Option<String>,
}

impl CalendarDefinitionRow {
    pub fn to_domain(
        &self,
        tracked_moons: Vec<CalendarTrackedMoon>,
    ) -> Result<CalendarDefinition, DbError> {
        let id = Uuid::parse_str(&self.id)?;
        let planet_id = Uuid::parse_str(&self.planet_id)?;
        let epoch = Duration::new(self.epoch_seconds_since_j2000);

        let structure = match self.structure_kind.as_str() {
            "SolarOnly" | "PureSolar" => CalendarStructureKind::SolarOnly,
            "LunarOnly" | "PureLunar" => CalendarStructureKind::LunarOnly,
            "Lunisolar" => CalendarStructureKind::Lunisolar,
            other => {
                return Err(DbError::Domain(DomainError::InvalidInvariant {
                    field: "structure_kind".to_string(),
                    reason: format!("unknown calendar structure kind: {}", other),
                }));
            }
        };

        let day_convention = match self.day_convention.as_str() {
            "Solar" => DayConvention::Solar,
            "Sidereal" => DayConvention::Sidereal,
            other => {
                return Err(DbError::Domain(DomainError::InvalidInvariant {
                    field: "day_convention".to_string(),
                    reason: format!("unknown day convention: {}", other),
                }));
            }
        };

        let year_convention = match self.year_convention.as_str() {
            "Sidereal" => YearConvention::Sidereal,
            "Tropical" => YearConvention::Tropical,
            "Anomalistic" => YearConvention::Anomalistic,
            other => {
                return Err(DbError::Domain(DomainError::InvalidInvariant {
                    field: "year_convention".to_string(),
                    reason: format!("unknown year convention: {}", other),
                }));
            }
        };

        let reference_moon = parse_optional_moon_reference(
            self.reference_moon_planet_id.clone(),
            self.reference_moon_minor_planet_id.clone(),
        )?;

        let def = CalendarDefinition::builder(id, planet_id, self.name.clone(), epoch)
            .with_structure(structure)
            .with_day_convention(day_convention)
            .with_year_convention(year_convention)
            .with_reference_moon(reference_moon)
            .with_founding_event_description(self.founding_event_description.clone())
            .with_tracked_moons(tracked_moons)
            .build()?;

        Ok(def)
    }
}
