use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::{validate_finite, validate_non_negative_finite};
use astronomicon_core::units::{Duration, Length, Temperature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeatShieldState {
    vehicle_component_id: Uuid,
    remaining_thickness: Length,
    surface_temperature: Temperature,
    captured_universe_epoch: Duration,
    captured_at_epoch: Duration,
}

impl HeatShieldState {
    pub fn new(
        vehicle_component_id: Uuid,
        remaining_thickness: Length,
        surface_temperature: Temperature,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        if vehicle_component_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "vehicle_component_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        validate_non_negative_finite(remaining_thickness.value(), "remaining_thickness")?;
        validate_non_negative_finite(surface_temperature.value(), "surface_temperature")?;
        validate_finite(captured_universe_epoch.value(), "captured_universe_epoch")?;
        validate_finite(captured_at_epoch.value(), "captured_at_epoch")?;

        Ok(Self {
            vehicle_component_id,
            remaining_thickness,
            surface_temperature,
            captured_universe_epoch,
            captured_at_epoch,
        })
    }

    pub fn vehicle_component_id(&self) -> Uuid {
        self.vehicle_component_id
    }

    pub fn remaining_thickness(&self) -> Length {
        self.remaining_thickness
    }

    pub fn remaining_thickness_m(&self) -> f64 {
        self.remaining_thickness.value()
    }

    pub fn surface_temperature(&self) -> Temperature {
        self.surface_temperature
    }

    pub fn surface_temperature_k(&self) -> f64 {
        self.surface_temperature.value()
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