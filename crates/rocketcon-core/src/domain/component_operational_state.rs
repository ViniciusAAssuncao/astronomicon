use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::{validate_finite, validate_unit_interval};
use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComponentOperationalState {
    vehicle_component_id: Uuid,
    load_fraction: f64,
    captured_universe_epoch: Duration,
    captured_at_epoch: Duration,
}

impl ComponentOperationalState {
    pub fn new(
        vehicle_component_id: Uuid,
        load_fraction: f64,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        validate_unit_interval(load_fraction, "load_fraction")?;
        validate_finite(captured_universe_epoch.value(), "captured_universe_epoch")?;
        validate_finite(captured_at_epoch.value(), "captured_at_epoch")?;

        Ok(Self {
            vehicle_component_id,
            load_fraction,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn vehicle_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn load_fraction(&self) -> f64 {
        self.load_fraction
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