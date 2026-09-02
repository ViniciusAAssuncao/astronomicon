use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::{validate_non_negative_finite, validate_positive_finite};
use astronomicon_core::units::{Mass, Speed, Volume};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PayloadSpecificationBuilder {
    component_id: Uuid,
    max_payload_mass: Mass,
    max_payload_volume: Volume,
    contained_vehicle_id: Option<Uuid>,
    generic_cargo_mass: Option<Mass>,
    separation_velocity: Speed,
}

impl PayloadSpecificationBuilder {
    pub fn new(
        component_id: Uuid,
        max_payload_mass: Mass,
        max_payload_volume: Volume,
        separation_velocity: Speed,
    ) -> Self {
        Self {
            component_id,
            max_payload_mass,
            max_payload_volume,
            contained_vehicle_id: None,
            generic_cargo_mass: None,
            separation_velocity,
        }
    }

    pub fn with_contained_vehicle_id(
        mut self,
        contained_vehicle_id: impl Into<Option<Uuid>>,
    ) -> Self {
        self.contained_vehicle_id = contained_vehicle_id.into();
        self
    }

    pub fn with_generic_cargo_mass(
        mut self,
        generic_cargo_mass: impl Into<Option<Mass>>,
    ) -> Self {
        self.generic_cargo_mass = generic_cargo_mass.into();
        self
    }

    pub fn build(self) -> RocketDomainResult<PayloadSpecification> {
        validate_positive_finite(self.max_payload_mass.value(), "max_payload_mass")?;
        validate_positive_finite(self.max_payload_volume.value(), "max_payload_volume")?;
        validate_non_negative_finite(self.separation_velocity.value(), "separation_velocity")?;

        if let Some(vid) = self.contained_vehicle_id {
            if vid.is_nil() {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "contained_vehicle_id".to_string(),
                    reason: "cannot be nil".to_string(),
                });
            }
        }

        if let Some(cargo) = self.generic_cargo_mass {
            validate_positive_finite(cargo.value(), "generic_cargo_mass")?;
            if cargo.value() > self.max_payload_mass.value() {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "generic_cargo_mass".to_string(),
                    reason: "generic_cargo_mass cannot exceed max_payload_mass".to_string(),
                });
            }
        }

        if self.contained_vehicle_id.is_some() && self.generic_cargo_mass.is_some() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "payload".to_string(),
                reason: "cannot contain both a vehicle and generic cargo simultaneously".to_string(),
            });
        }

        Ok(PayloadSpecification {
            component_id: self.component_id,
            max_payload_mass: self.max_payload_mass,
            max_payload_volume: self.max_payload_volume,
            contained_vehicle_id: self.contained_vehicle_id,
            generic_cargo_mass: self.generic_cargo_mass,
            separation_velocity: self.separation_velocity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayloadSpecification {
    component_id: Uuid,
    max_payload_mass: Mass,
    max_payload_volume: Volume,
    contained_vehicle_id: Option<Uuid>,
    generic_cargo_mass: Option<Mass>,
    separation_velocity: Speed,
}

impl PayloadSpecification {
    pub fn builder(
        component_id: Uuid,
        max_payload_mass: Mass,
        max_payload_volume: Volume,
        separation_velocity: Speed,
    ) -> PayloadSpecificationBuilder {
        PayloadSpecificationBuilder::new(
            component_id,
            max_payload_mass,
            max_payload_volume,
            separation_velocity,
        )
    }

    pub fn new(
        component_id: Uuid,
        max_payload_mass: Mass,
        max_payload_volume: Volume,
        contained_vehicle_id: Option<Uuid>,
        generic_cargo_mass: Option<Mass>,
        separation_velocity: Speed,
    ) -> RocketDomainResult<Self> {
        Self::builder(
            component_id,
            max_payload_mass,
            max_payload_volume,
            separation_velocity,
        )
        .with_contained_vehicle_id(contained_vehicle_id)
        .with_generic_cargo_mass(generic_cargo_mass)
        .build()
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn max_payload_mass(&self) -> Mass {
        self.max_payload_mass
    }

    pub fn max_payload_volume(&self) -> Volume {
        self.max_payload_volume
    }

    pub fn contained_vehicle_id(&self) -> Option<Uuid> {
        self.contained_vehicle_id
    }

    pub fn generic_cargo_mass(&self) -> Option<Mass> {
        self.generic_cargo_mass
    }

    pub fn separation_velocity(&self) -> Speed {
        self.separation_velocity
    }

    pub fn has_vehicle(&self) -> bool {
        self.contained_vehicle_id.is_some()
    }

    pub fn has_cargo(&self) -> bool {
        self.generic_cargo_mass.is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.contained_vehicle_id.is_none() && self.generic_cargo_mass.is_none()
    }
}