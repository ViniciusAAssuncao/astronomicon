use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::Mass;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropellantTankSpecification {
    component_id: Uuid,
    propellant_id: Uuid,
    max_propellant_mass: Mass,
}

impl PropellantTankSpecification {
    pub fn new(
        component_id: Uuid,
        propellant_id: Uuid,
        max_propellant_mass: Mass,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(max_propellant_mass.value(), "max_propellant_mass")?;

        Ok(Self {
            component_id,
            propellant_id,
            max_propellant_mass,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn propellant_id(&self) -> Uuid {
        self.propellant_id
    }

    pub fn max_propellant_mass(&self) -> Mass {
        self.max_propellant_mass
    }
}
