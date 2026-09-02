use astronomicon_core::units::Angle;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ActuatorControlCommand {
    pub target_gimbal_pitch: Option<Angle>,
    pub target_gimbal_yaw: Option<Angle>,
    pub target_reaction_wheel_torque_fraction: Option<f64>,
}

impl ActuatorControlCommand {
    pub fn new(
        target_gimbal_pitch: Option<Angle>,
        target_gimbal_yaw: Option<Angle>,
        target_reaction_wheel_torque_fraction: Option<f64>,
    ) -> Self {
        Self {
            target_gimbal_pitch,
            target_gimbal_yaw,
            target_reaction_wheel_torque_fraction,
        }
    }

    pub fn with_gimbal(target_pitch: Angle, target_yaw: Angle) -> Self {
        Self {
            target_gimbal_pitch: Some(target_pitch),
            target_gimbal_yaw: Some(target_yaw),
            target_reaction_wheel_torque_fraction: None,
        }
    }

    pub fn with_reaction_wheel(fraction: f64) -> Self {
        Self {
            target_gimbal_pitch: None,
            target_gimbal_yaw: None,
            target_reaction_wheel_torque_fraction: Some(fraction),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VehicleControlInput {
    pub actuator_commands: HashMap<Uuid, ActuatorControlCommand>,
}

impl VehicleControlInput {
    pub fn new() -> Self {
        Self {
            actuator_commands: HashMap::new(),
        }
    }

    pub fn with_command(mut self, vehicle_component_id: Uuid, command: ActuatorControlCommand) -> Self {
        self.actuator_commands.insert(vehicle_component_id, command);
        self
    }

    pub fn command_for(&self, vehicle_component_id: &Uuid) -> Option<&ActuatorControlCommand> {
        self.actuator_commands.get(vehicle_component_id)
    }
}
