use crate::domain::VehiclePhysicalState;
use astronomicon_core::units::{
    AccelerationVector, AngularAccelerationVector, AngularVelocityVector, Position, Quaternion,
    VelocityVector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RigidBodyState {
    pub position: Position,
    pub velocity: VelocityVector,
    pub orientation: Quaternion,
    pub angular_velocity: AngularVelocityVector,
}

impl RigidBodyState {
    pub fn new(
        position: Position,
        velocity: VelocityVector,
        orientation: Quaternion,
        angular_velocity: AngularVelocityVector,
    ) -> Self {
        Self {
            position,
            velocity,
            orientation,
            angular_velocity,
        }
    }

    pub fn from_physical_state(state: &VehiclePhysicalState) -> Self {
        Self {
            position: state.position(),
            velocity: state.velocity(),
            orientation: state.orientation(),
            angular_velocity: state.angular_velocity(),
        }
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
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RigidBodyDerivative {
    pub velocity: VelocityVector,
    pub acceleration: AccelerationVector,
    pub angular_velocity: AngularVelocityVector,
    pub angular_acceleration: AngularAccelerationVector,
}

impl RigidBodyDerivative {
    pub fn new(
        velocity: VelocityVector,
        acceleration: AccelerationVector,
        angular_velocity: AngularVelocityVector,
        angular_acceleration: AngularAccelerationVector,
    ) -> Self {
        Self {
            velocity,
            acceleration,
            angular_velocity,
            angular_acceleration,
        }
    }

    pub fn velocity(&self) -> VelocityVector {
        self.velocity
    }

    pub fn acceleration(&self) -> AccelerationVector {
        self.acceleration
    }

    pub fn angular_velocity(&self) -> AngularVelocityVector {
        self.angular_velocity
    }

    pub fn angular_acceleration(&self) -> AngularAccelerationVector {
        self.angular_acceleration
    }
}
