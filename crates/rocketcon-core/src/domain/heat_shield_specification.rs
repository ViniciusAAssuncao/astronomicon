use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::Length;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeatShieldSpecification {
    component_id: Uuid,
    material_id: Uuid,
    shield_thickness: Length,
}

impl HeatShieldSpecification {
    pub fn new(
        component_id: Uuid,
        material_id: Uuid,
        shield_thickness: Length,
    ) -> RocketDomainResult<Self> {
        if component_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "component_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        if material_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "material_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        validate_positive_finite(shield_thickness.value(), "shield_thickness")?;

        Ok(Self {
            component_id,
            material_id,
            shield_thickness,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn material_id(&self) -> Uuid {
        self.material_id
    }

    pub fn shield_thickness(&self) -> Length {
        self.shield_thickness
    }

    pub fn shield_thickness_m(&self) -> f64 {
        self.shield_thickness.value()
    }
}