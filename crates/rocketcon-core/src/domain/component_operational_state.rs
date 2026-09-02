use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::{validate_finite, validate_unit_interval};
use astronomicon_core::units::{Angle, Duration};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComponentOperationalState {
    vehicle_component_id: Uuid,
    load_fraction: f64,
    current_gimbal_pitch: Option<Angle>,
    current_gimbal_yaw: Option<Angle>,
    captured_universe_epoch: Duration,
    captured_at_epoch: Duration,
}

impl ComponentOperationalState {
    pub fn new(
        vehicle_component_id: Uuid,
        load_fraction: f64,
        current_gimbal_pitch: Option<Angle>,
        current_gimbal_yaw: Option<Angle>,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        validate_unit_interval(load_fraction, "load_fraction")?;
        if let Some(pitch) = current_gimbal_pitch {
            validate_finite(pitch.value(), "current_gimbal_pitch")?;
        }
        if let Some(yaw) = current_gimbal_yaw {
            validate_finite(yaw.value(), "current_gimbal_yaw")?;
        }
        validate_finite(captured_universe_epoch.value(), "captured_universe_epoch")?;
        validate_finite(captured_at_epoch.value(), "captured_at_epoch")?;

        Ok(Self {
            vehicle_component_id,
            load_fraction,
            current_gimbal_pitch,
            current_gimbal_yaw,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn new_simple(
        vehicle_component_id: Uuid,
        load_fraction: f64,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        Self::new(
            vehicle_component_id,
            load_fraction,
            None,
            None,
            captured_universe_epoch,
            captured_at_epoch,
        )
    }

    pub fn vehicle_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn load_fraction(&self) -> f64 {
        self.load_fraction
    }

    pub fn current_gimbal_pitch(&self) -> Option<Angle> {
        self.current_gimbal_pitch
    }

    pub fn current_gimbal_yaw(&self) -> Option<Angle> {
        self.current_gimbal_yaw
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
