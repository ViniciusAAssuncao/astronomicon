use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CalendarStructureKind {
    SolarOnly,
    LunarOnly,
    Lunisolar,
}

impl CalendarStructureKind {
    pub fn is_solar_only(&self) -> bool {
        matches!(self, Self::SolarOnly)
    }

    pub fn is_lunar_only(&self) -> bool {
        matches!(self, Self::LunarOnly)
    }

    pub fn is_lunisolar(&self) -> bool {
        matches!(self, Self::Lunisolar)
    }

    pub fn requires_reference_moon(&self) -> bool {
        matches!(self, Self::LunarOnly | Self::Lunisolar)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DayConvention {
    Solar,
    Sidereal,
}

impl DayConvention {
    pub fn is_solar(&self) -> bool {
        matches!(self, Self::Solar)
    }

    pub fn is_sidereal(&self) -> bool {
        matches!(self, Self::Sidereal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum YearConvention {
    Sidereal,
    Tropical,
    Anomalistic,
}

impl YearConvention {
    pub fn is_sidereal(&self) -> bool {
        matches!(self, Self::Sidereal)
    }

    pub fn is_tropical(&self) -> bool {
        matches!(self, Self::Tropical)
    }

    pub fn is_anomalistic(&self) -> bool {
        matches!(self, Self::Anomalistic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CalendarMoonReference {
    Planet(Uuid),
    MinorPlanet(Uuid),
}

impl CalendarMoonReference {
    pub fn id(&self) -> Uuid {
        match self {
            Self::Planet(id) | Self::MinorPlanet(id) => *id,
        }
    }

    pub fn is_planet(&self) -> bool {
        matches!(self, Self::Planet(_))
    }

    pub fn is_minor_planet(&self) -> bool {
        matches!(self, Self::MinorPlanet(_))
    }
}
