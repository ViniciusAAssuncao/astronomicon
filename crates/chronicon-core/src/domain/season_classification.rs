use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeasonalityClassification {
    Aseasonal,
    TidallyLockedAseasonal,
    TidallyLockedDistanceDriven,
    ObliquityDominated,
    EccentricityDominated,
    Combined,
}

impl SeasonalityClassification {
    pub fn is_aseasonal(&self) -> bool {
        matches!(self, Self::Aseasonal | Self::TidallyLockedAseasonal)
    }

    pub fn is_tidally_locked(&self) -> bool {
        matches!(
            self,
            Self::TidallyLockedAseasonal | Self::TidallyLockedDistanceDriven
        )
    }

    pub fn has_seasons(&self) -> bool {
        !self.is_aseasonal()
    }
}