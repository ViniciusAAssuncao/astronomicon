use crate::domain::calendar_conventions::{
    CalendarMoonReference, CalendarStructureKind, DayConvention, YearConvention,
};
use crate::domain::calendar_tracked_moon::CalendarTrackedMoon;
use crate::domain::validation::{validate_finite, validate_not_empty};
use crate::error::{ChronosError, ChronosResult};
use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarDefinition {
    pub id: Uuid,
    pub planet_id: Uuid,
    pub name: String,
    pub structure: CalendarStructureKind,
    pub epoch: Duration,
    pub day_convention: DayConvention,
    pub year_convention: YearConvention,
    pub reference_moon: Option<CalendarMoonReference>,
    pub founding_event_description: Option<String>,
    pub tracked_moons: Vec<CalendarTrackedMoon>,
}

impl CalendarDefinition {
    pub fn new(
        id: Uuid,
        planet_id: Uuid,
        name: String,
        structure: CalendarStructureKind,
        epoch: Duration,
        day_convention: DayConvention,
        year_convention: YearConvention,
        reference_moon: Option<CalendarMoonReference>,
        founding_event_description: Option<String>,
        tracked_moons: Vec<CalendarTrackedMoon>,
    ) -> ChronosResult<Self> {
        validate_not_empty(&name, "name")?;
        validate_finite(epoch.value(), "epoch")?;

        if structure.requires_reference_moon() && reference_moon.is_none() {
            return Err(ChronosError::InvalidInvariant {
                field: "reference_moon".to_string(),
                reason: "calendar structure requires a reference moon".to_string(),
            });
        }

        Ok(Self {
            id,
            planet_id,
            name,
            structure,
            epoch,
            day_convention,
            year_convention,
            reference_moon,
            founding_event_description,
            tracked_moons,
        })
    }

    pub fn builder(
        id: Uuid,
        planet_id: Uuid,
        name: impl Into<String>,
        epoch: Duration,
    ) -> CalendarDefinitionBuilder {
        CalendarDefinitionBuilder::new(id, planet_id, name, epoch)
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn planet_id(&self) -> Uuid {
        self.planet_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn structure(&self) -> CalendarStructureKind {
        self.structure
    }

    pub fn epoch(&self) -> Duration {
        self.epoch
    }

    pub fn day_convention(&self) -> DayConvention {
        self.day_convention
    }

    pub fn year_convention(&self) -> YearConvention {
        self.year_convention
    }

    pub fn reference_moon(&self) -> Option<CalendarMoonReference> {
        self.reference_moon
    }

    pub fn founding_event_description(&self) -> Option<&str> {
        self.founding_event_description.as_deref()
    }

    pub fn tracked_moons(&self) -> &[CalendarTrackedMoon] {
        &self.tracked_moons
    }
}

pub struct CalendarDefinitionBuilder {
    id: Uuid,
    planet_id: Uuid,
    name: String,
    structure: CalendarStructureKind,
    epoch: Duration,
    day_convention: DayConvention,
    year_convention: YearConvention,
    reference_moon: Option<CalendarMoonReference>,
    founding_event_description: Option<String>,
    tracked_moons: Vec<CalendarTrackedMoon>,
}

impl CalendarDefinitionBuilder {
    pub fn new(
        id: Uuid,
        planet_id: Uuid,
        name: impl Into<String>,
        epoch: Duration,
    ) -> Self {
        Self {
            id,
            planet_id,
            name: name.into(),
            structure: CalendarStructureKind::SolarOnly,
            epoch,
            day_convention: DayConvention::Solar,
            year_convention: YearConvention::Tropical,
            reference_moon: None,
            founding_event_description: None,
            tracked_moons: Vec::new(),
        }
    }

    pub fn with_structure(mut self, structure: CalendarStructureKind) -> Self {
        self.structure = structure;
        self
    }

    pub fn with_day_convention(mut self, convention: DayConvention) -> Self {
        self.day_convention = convention;
        self
    }

    pub fn with_year_convention(mut self, convention: YearConvention) -> Self {
        self.year_convention = convention;
        self
    }

    pub fn with_reference_moon(mut self, moon: Option<CalendarMoonReference>) -> Self {
        self.reference_moon = moon;
        self
    }

    pub fn with_founding_event_description(mut self, desc: Option<String>) -> Self {
        self.founding_event_description = desc;
        self
    }

    pub fn with_tracked_moons(mut self, moons: Vec<CalendarTrackedMoon>) -> Self {
        self.tracked_moons = moons;
        self
    }

    pub fn build(self) -> ChronosResult<CalendarDefinition> {
        CalendarDefinition::new(
            self.id,
            self.planet_id,
            self.name,
            self.structure,
            self.epoch,
            self.day_convention,
            self.year_convention,
            self.reference_moon,
            self.founding_event_description,
            self.tracked_moons,
        )
    }
}
