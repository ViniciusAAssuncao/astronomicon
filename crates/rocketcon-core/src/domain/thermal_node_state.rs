use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::{validate_finite, validate_non_negative_finite};
use astronomicon_core::units::{Duration, Temperature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermalNodeState {
    vehicle_component_id: Uuid,
    current_temperature: Temperature,
    captured_universe_epoch: Duration,
    captured_at_epoch: Duration,
}

impl ThermalNodeState {
    pub fn new(
        vehicle_component_id: Uuid,
        current_temperature: Temperature,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        if vehicle_component_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "vehicle_component_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        validate_non_negative_finite(current_temperature.value(), "current_temperature")?;
        validate_finite(captured_universe_epoch.value(), "captured_universe_epoch")?;
        validate_finite(captured_at_epoch.value(), "captured_at_epoch")?;

        Ok(Self {
            vehicle_component_id,
            current_temperature,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn vehicle_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn current_temperature(&self) -> Temperature {
        self.current_temperature
    }

    pub fn current_temperature_k(&self) -> f64 {
        self.current_temperature.value()
    }

    pub fn captured_universe_epoch(&self) -> Duration {
        self.captured_universe_epoch
    }

    pub fn captured_at_epoch(&self) -> Duration {
        self.captured_at_epoch
    }

    pub fn captured_total_epoch(&self) -> Duration {
        self.captured_universe_epoch + self.captured_at_epoch
    }
}
