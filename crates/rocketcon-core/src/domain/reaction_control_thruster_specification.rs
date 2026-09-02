use crate::domain::thrust_producer::ThrustProducer;
use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::{Duration, Force, Impulse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactionControlThrusterSpecification {
    component_id: Uuid,
    propellant_id: Uuid,
    specific_impulse_vacuum: Duration,
    max_thrust: Force,
    min_impulse_bit: Option<Impulse>,
}

impl ReactionControlThrusterSpecification {
    pub fn new(
        component_id: Uuid,
        propellant_id: Uuid,
        specific_impulse_vacuum: Duration,
        max_thrust: Force,
        min_impulse_bit: Option<Impulse>,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(specific_impulse_vacuum.value(), "specific_impulse_vacuum")?;
        validate_positive_finite(max_thrust.value(), "max_thrust")?;

        if let Some(bit) = min_impulse_bit {
            validate_positive_finite(bit.value(), "min_impulse_bit")?;
        }

        Ok(Self {
            component_id,
            propellant_id,
            specific_impulse_vacuum,
            max_thrust,
            min_impulse_bit,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn propellant_id(&self) -> Uuid {
        self.propellant_id
    }

    pub fn specific_impulse_vacuum(&self) -> Duration {
        self.specific_impulse_vacuum
    }

    pub fn max_thrust(&self) -> Force {
        self.max_thrust
    }

    pub fn min_impulse_bit(&self) -> Option<Impulse> {
        self.min_impulse_bit
    }

    pub fn is_pulsed(&self) -> bool {
        self.min_impulse_bit.is_some()
    }
}

impl ThrustProducer for ReactionControlThrusterSpecification {
    fn specific_impulse_vacuum(&self) -> Duration {
        self.specific_impulse_vacuum
    }

    fn max_thrust(&self) -> Force {
        self.max_thrust
    }
}
