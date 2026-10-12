use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::{validate_finite, validate_positive_finite};
use astronomicon_core::units::{
    AngularVelocity, AngularVelocityVector, Duration, Position, Pressure, Quaternion, Speed,
    VelocityVector,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehiclePhysicalState {
    vehicle_id: Uuid,
    position: Position,
    velocity: VelocityVector,
    orientation: Quaternion,
    angular_velocity: AngularVelocityVector,
    reference_body_id: Uuid,
    captured_universe_epoch: Duration,
    captured_at_epoch: Duration,
    max_dynamic_pressure: Option<Pressure>,
    max_dynamic_pressure_epoch: Option<Duration>,
}

impl VehiclePhysicalState {
    pub fn new(
        vehicle_id: Uuid,
        position: Position,
        velocity: VelocityVector,
        orientation: Quaternion,
        angular_velocity: AngularVelocityVector,
        reference_body_id: Uuid,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
    ) -> RocketDomainResult<Self> {
        Self::new_with_max_q(
            vehicle_id,
            position,
            velocity,
            orientation,
            angular_velocity,
            reference_body_id,
            captured_universe_epoch,
            captured_at_epoch,
            None,
            None,
        )
    }

    pub fn new_with_max_q(
        vehicle_id: Uuid,
        position: Position,
        velocity: VelocityVector,
        orientation: Quaternion,
        angular_velocity: AngularVelocityVector,
        reference_body_id: Uuid,
        captured_universe_epoch: Duration,
        captured_at_epoch: Duration,
        max_dynamic_pressure: Option<Pressure>,
        max_dynamic_pressure_epoch: Option<Duration>,
    ) -> RocketDomainResult<Self> {
        if vehicle_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "vehicle_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }

        if reference_body_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "reference_body_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }

        validate_finite(position.raw().0, "position_x")?;
        validate_finite(position.raw().1, "position_y")?;
        validate_finite(position.raw().2, "position_z")?;

        validate_finite(velocity.raw().0, "velocity_x")?;
        validate_finite(velocity.raw().1, "velocity_y")?;
        validate_finite(velocity.raw().2, "velocity_z")?;

        validate_finite(orientation.w(), "orientation_q_w")?;
        validate_finite(orientation.x(), "orientation_q_x")?;
        validate_finite(orientation.y(), "orientation_q_y")?;
        validate_finite(orientation.z(), "orientation_q_z")?;

        if !orientation.is_normalized(1e-4) {
            return Err(RocketDomainError::InvalidInvariant {
                field: "orientation".to_string(),
                reason: "quaternion must be normalized".to_string(),
            });
        }

        validate_finite(angular_velocity.raw().0, "angular_velocity_x")?;
        validate_finite(angular_velocity.raw().1, "angular_velocity_y")?;
        validate_finite(angular_velocity.raw().2, "angular_velocity_z")?;

        validate_positive_finite(captured_universe_epoch.value(), "captured_universe_epoch")?;
        validate_positive_finite(captured_at_epoch.value(), "captured_at_epoch")?;

        if let Some(q) = max_dynamic_pressure {
            validate_finite(q.value(), "max_dynamic_pressure")?;
        }
        if let Some(ep) = max_dynamic_pressure_epoch {
            validate_finite(ep.value(), "max_dynamic_pressure_epoch")?;
        }

        Ok(Self {
            vehicle_id,
            position,
            velocity,
            orientation,
            angular_velocity,
            reference_body_id,
            captured_universe_epoch,
            captured_at_epoch,
            max_dynamic_pressure,
            max_dynamic_pressure_epoch,
        })
    }

    pub fn vehicle_id(&self) -> Uuid {
        self.vehicle_id
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn velocity(&self) -> VelocityVector {
        self.velocity
    }

    pub fn orientation(&self) -> Quaternion {
        self.orientation
    }

    pub fn angular_velocity(&self) -> AngularVelocityVector {
        self.angular_velocity
    }

    pub fn reference_body_id(&self) -> Uuid {
        self.reference_body_id
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

    pub fn speed(&self) -> Speed {
        self.velocity.magnitude()
    }

    pub fn angular_speed(&self) -> AngularVelocity {
        self.angular_velocity.magnitude()
    }

    pub fn max_dynamic_pressure(&self) -> Option<Pressure> {
        self.max_dynamic_pressure
    }

    pub fn max_q(&self) -> Option<Pressure> {
        self.max_dynamic_pressure
    }

    pub fn max_dynamic_pressure_epoch(&self) -> Option<Duration> {
        self.max_dynamic_pressure_epoch
    }

    pub fn max_q_epoch(&self) -> Option<Duration> {
        self.max_dynamic_pressure_epoch
    }

    pub fn with_max_dynamic_pressure(
        mut self,
        max_dynamic_pressure: Option<Pressure>,
        max_dynamic_pressure_epoch: Option<Duration>,
    ) -> Self {
        self.max_dynamic_pressure = max_dynamic_pressure;
        self.max_dynamic_pressure_epoch = max_dynamic_pressure_epoch;
        self
    }

    pub fn with_max_q(
        self,
        max_q: Option<Pressure>,
        max_q_epoch: Option<Duration>,
    ) -> Self {
        self.with_max_dynamic_pressure(max_q, max_q_epoch)
    }
}
