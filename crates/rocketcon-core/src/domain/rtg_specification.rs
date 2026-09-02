use crate::error::{RocketDomainError, RocketDomainResult};
use crate::physics_reference::RadioisotopeType;
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::{Duration, Mass};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RtgSpecification {
    component_id: Uuid,
    radioisotope: RadioisotopeType,
    fuel_mass: Mass,
    conversion_efficiency: f64,
    fuel_loaded_universe_epoch: Duration,
}

impl RtgSpecification {
    pub fn new(
        component_id: Uuid,
        radioisotope: RadioisotopeType,
        fuel_mass: Mass,
        conversion_efficiency: f64,
        fuel_loaded_universe_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(fuel_mass.value(), "fuel_mass")?;

        if !conversion_efficiency.is_finite()
            || conversion_efficiency <= 0.0
            || conversion_efficiency > 1.0
        {
            return Err(RocketDomainError::InvalidInvariant {
                field: "conversion_efficiency".to_string(),
                reason: "must be in range (0, 1]".to_string(),
            });
        }

        if !fuel_loaded_universe_epoch.value().is_finite()
            || fuel_loaded_universe_epoch.value() < 0.0
        {
            return Err(RocketDomainError::InvalidInvariant {
                field: "fuel_loaded_universe_epoch".to_string(),
                reason: "must be finite and non-negative".to_string(),
            });
        }

        Ok(Self {
            component_id,
            radioisotope,
            fuel_mass,
            conversion_efficiency,
            fuel_loaded_universe_epoch,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn radioisotope(&self) -> RadioisotopeType {
        self.radioisotope
    }

    pub fn fuel_mass(&self) -> Mass {
        self.fuel_mass
    }

    pub fn conversion_efficiency(&self) -> f64 {
        self.conversion_efficiency
    }

    pub fn fuel_loaded_universe_epoch(&self) -> Duration {
        self.fuel_loaded_universe_epoch
    }
}