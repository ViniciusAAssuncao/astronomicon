use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::validate_finite;
use astronomicon_core::units::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComponentPayloadState {
    vehicle_component_id: Uuid,
    is_deployed: bool,
    captured_universe_epoch: Duration,
    captured_at_epoch: Duration,
}

impl ComponentPayloadState {
    pub fn new(
        vehicle_component_id: Uuid,
        is_deployed: bool,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        validate_finite(captured_universe_epoch.value(), "captured_universe_epoch")?;
        validate_finite(captured_at_epoch.value(), "captured_at_epoch")?;

        Ok(Self {
            vehicle_component_id,
            is_deployed,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn vehicle_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn is_deployed(&self) -> bool {
        self.is_deployed
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