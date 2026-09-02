use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::validate_non_negative_finite;
use astronomicon_core::units::Mass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PayloadDeploymentEvent {
    vehicle_component_id: Uuid,
    ejected_mass: Mass,
    deployed_vehicle_id: Option<Uuid>,
}

impl PayloadDeploymentEvent {
    pub fn new(vehicle_component_id: Uuid, ejected_mass: Mass) -> RocketDomainResult<Self> {
        if vehicle_component_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "vehicle_component_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        validate_non_negative_finite(ejected_mass.value(), "ejected_mass")?;
        Ok(Self {
            vehicle_component_id,
            ejected_mass,
            deployed_vehicle_id: None,
        })
    }

    pub fn new_with_vehicle(
        vehicle_component_id: Uuid,
        ejected_mass: Mass,
        deployed_vehicle_id: Option<Uuid>,
    ) -> RocketDomainResult<Self> {
        if vehicle_component_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "vehicle_component_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        validate_non_negative_finite(ejected_mass.value(), "ejected_mass")?;
        Ok(Self {
            vehicle_component_id,
            ejected_mass,
            deployed_vehicle_id,
        })
    }

    pub fn with_deployed_vehicle_id(mut self, deployed_vehicle_id: impl Into<Option<Uuid>>) -> Self {
        self.deployed_vehicle_id = deployed_vehicle_id.into();
        self
    }

    pub fn vehicle_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn payload_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn ejected_mass(&self) -> Mass {
        self.ejected_mass
    }

    pub fn deployed_vehicle_id(&self) -> Option<Uuid> {
        self.deployed_vehicle_id
    }
}