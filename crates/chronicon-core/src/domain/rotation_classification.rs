use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RotationClassification {
    Prograde,
    Retrograde,
    Synchronous,
    NoRotationalData,
}

impl RotationClassification {
    pub fn is_prograde(&self) -> bool {
        matches!(self, Self::Prograde)
    }

    pub fn is_retrograde(&self) -> bool {
        matches!(self, Self::Retrograde)
    }

    pub fn is_synchronous(&self) -> bool {
        matches!(self, Self::Synchronous)
    }

    pub fn has_data(&self) -> bool {
        !matches!(self, Self::NoRotationalData)
    }
}