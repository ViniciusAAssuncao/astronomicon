use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::{Energy, Luminosity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatterySpecification {
    component_id: Uuid,
    capacity: Energy,
    max_discharge_power: Luminosity,
    max_charge_power: Option<Luminosity>,
}

impl BatterySpecification {
    pub fn new(
        component_id: Uuid,
        capacity: Energy,
        max_discharge_power: Luminosity,
        max_charge_power: Option<Luminosity>,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(capacity.value(), "capacity")?;
        validate_positive_finite(max_discharge_power.value(), "max_discharge_power")?;

        if let Some(cp) = max_charge_power {
            validate_positive_finite(cp.value(), "max_charge_power")?;
        }

        Ok(Self {
            component_id,
            capacity,
            max_discharge_power,
            max_charge_power,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn capacity(&self) -> Energy {
        self.capacity
    }

    pub fn max_discharge_power(&self) -> Luminosity {
        self.max_discharge_power
    }

    pub fn max_charge_power(&self) -> Option<Luminosity> {
        self.max_charge_power
    }

    pub fn is_rechargeable(&self) -> bool {
        self.max_charge_power.is_some()
    }
}
