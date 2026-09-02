use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::{AngularMomentum, Torque};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionWheelSpecification {
    component_id: Uuid,
    max_torque: Torque,
    max_angular_momentum_storage: AngularMomentum,
}

impl ReactionWheelSpecification {
    pub fn new(
        component_id: Uuid,
        max_torque: Torque,
        max_angular_momentum_storage: AngularMomentum,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(max_torque.value(), "max_torque")?;
        validate_positive_finite(
            max_angular_momentum_storage.value(),
            "max_angular_momentum_storage",
        )?;

        Ok(Self {
            component_id,
            max_torque,
            max_angular_momentum_storage,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn max_torque(&self) -> Torque {
        self.max_torque
    }

    pub fn max_angular_momentum_storage(&self) -> AngularMomentum {
        self.max_angular_momentum_storage
    }
}
