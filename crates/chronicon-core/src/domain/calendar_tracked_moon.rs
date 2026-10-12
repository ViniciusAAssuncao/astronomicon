use crate::domain::calendar_conventions::CalendarMoonReference;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CalendarTrackedMoon {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub moon: CalendarMoonReference,
}

impl CalendarTrackedMoon {
    pub fn new(id: Uuid, calendar_id: Uuid, moon: CalendarMoonReference) -> Self {
        Self {
            id,
            calendar_id,
            moon,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn calendar_id(&self) -> Uuid {
        self.calendar_id
    }

    pub fn moon(&self) -> CalendarMoonReference {
        self.moon
    }
}