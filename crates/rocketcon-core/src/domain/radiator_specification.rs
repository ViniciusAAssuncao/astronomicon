use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::validate_positive_finite;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadiatorSpecification {
    component_id: Uuid,
    radiating_area_m2: f64,
    emissivity: f64,
    solar_absorptivity: f64,
}

impl RadiatorSpecification {
    pub fn new(
        component_id: Uuid,
        radiating_area_m2: f64,
        emissivity: f64,
        solar_absorptivity: f64,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(radiating_area_m2, "radiating_area_m2")?;

        if !emissivity.is_finite() || emissivity <= 0.0 || emissivity > 1.0 {
            return Err(RocketDomainError::InvalidInvariant {
                field: "emissivity".to_string(),
                reason: "must be in range (0, 1]".to_string(),
            });
        }

        if !solar_absorptivity.is_finite() || solar_absorptivity <= 0.0 || solar_absorptivity > 1.0 {
            return Err(RocketDomainError::InvalidInvariant {
                field: "solar_absorptivity".to_string(),
                reason: "must be in range (0, 1]".to_string(),
            });
        }

        Ok(Self {
            component_id,
            radiating_area_m2,
            emissivity,
            solar_absorptivity,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn radiating_area_m2(&self) -> f64 {
        self.radiating_area_m2
    }

    pub fn emissivity(&self) -> f64 {
        self.emissivity
    }

    pub fn solar_absorptivity(&self) -> f64 {
        self.solar_absorptivity
    }
}