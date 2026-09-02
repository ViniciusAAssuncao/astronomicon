use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::{validate_finite, validate_non_negative_finite};
use astronomicon_core::units::{Duration, Energy};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnergyReservoirState {
    vehicle_component_id: Uuid,
    stored_energy: Energy,
    captured_universe_epoch: Duration,
    captured_at_epoch: Duration,
}

impl EnergyReservoirState {
    pub fn new(
        vehicle_component_id: Uuid,
        stored_energy: Energy,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        validate_non_negative_finite(stored_energy.value(), "stored_energy")?;
        validate_finite(captured_universe_epoch.value(), "captured_universe_epoch")?;
        validate_finite(captured_at_epoch.value(), "captured_at_epoch")?;

        Ok(Self {
            vehicle_component_id,
            stored_energy,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn vehicle_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn stored_energy(&self) -> Energy {
        self.stored_energy
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