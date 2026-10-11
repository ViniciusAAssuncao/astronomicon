use astronomicon_core::units::{Angle, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ActuatorControlCommand {
    pub target_gimbal_pitch: Option<Angle>,
    pub target_gimbal_yaw: Option<Angle>,
    pub target_reaction_wheel_torque_fraction: Option<f64>,
    pub target_rcs_throttle: Option<f64>,
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
            target_rcs_throttle: None,
        }
    }

    pub fn with_rcs_throttle(mut self, throttle: f64) -> Self {
        self.target_rcs_throttle = Some(throttle);
        self
    }

    pub fn with_gimbal(target_pitch: Angle, target_yaw: Angle) -> Self {
        Self {
            target_gimbal_pitch: Some(target_pitch),
            target_gimbal_yaw: Some(target_yaw),
            target_reaction_wheel_torque_fraction: None,
            target_rcs_throttle: None,
        }
    }

    pub fn with_reaction_wheel(fraction: f64) -> Self {
        Self {
            target_gimbal_pitch: None,
            target_gimbal_yaw: None,
            target_reaction_wheel_torque_fraction: Some(fraction),
            target_rcs_throttle: None,
        }
    }

    pub fn rcs(throttle: f64) -> Self {
        Self {
            target_gimbal_pitch: None,
            target_gimbal_yaw: None,
            target_reaction_wheel_torque_fraction: None,
            target_rcs_throttle: Some(throttle),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct VehicleControlInput {
    pub actuator_commands: HashMap<Uuid, ActuatorControlCommand>,
    pub target_attitude_torque: Option<Vector3>,
    pub target_translation_force: Option<Vector3>,
    pub pitch: Option<f64>,
    pub yaw: Option<f64>,
    pub roll: Option<f64>,
}

impl VehicleControlInput {
    pub fn new() -> Self {
        Self {
            actuator_commands: HashMap::new(),
            target_attitude_torque: None,
            target_translation_force: None,
            pitch: None,
            yaw: None,
            roll: None,
        }
    }

    pub fn with_command(mut self, vehicle_component_id: Uuid, command: ActuatorControlCommand) -> Self {
        self.actuator_commands.insert(vehicle_component_id, command);
        self
    }

    pub fn with_attitude_torque(mut self, torque: Vector3) -> Self {
        self.target_attitude_torque = Some(torque);
        self
    }

    pub fn with_translation_force(mut self, force: Vector3) -> Self {
        self.target_translation_force = Some(force);
        self
    }

    pub fn with_pitch_yaw_roll(mut self, pitch: f64, yaw: f64, roll: f64) -> Self {
        self.pitch = Some(pitch);
        self.yaw = Some(yaw);
        self.roll = Some(roll);
        self
    }

    pub fn with_pitch(mut self, pitch: f64) -> Self {
        self.pitch = Some(pitch);
        self
    }

    pub fn with_yaw(mut self, yaw: f64) -> Self {
        self.yaw = Some(yaw);
        self
    }

    pub fn with_roll(mut self, roll: f64) -> Self {
        self.roll = Some(roll);
        self
    }

    pub fn command_for(&self, vehicle_component_id: &Uuid) -> Option<&ActuatorControlCommand> {
        self.actuator_commands.get(vehicle_component_id)
    }

    pub fn has_attitude_demand(&self) -> bool {
        self.target_attitude_torque.is_some()
            || self.pitch.is_some()
            || self.yaw.is_some()
            || self.roll.is_some()
    }

    pub fn attitude_demand_vector(&self) -> Option<Vector3> {
        if let Some(t) = self.target_attitude_torque {
            return Some(t);
        }
        if self.pitch.is_some() || self.yaw.is_some() || self.roll.is_some() {
            let p = self.pitch.unwrap_or(0.0);
            let y = self.yaw.unwrap_or(0.0);
            let r = self.roll.unwrap_or(0.0);
            return Some(Vector3::new(p, y, r));
        }
        None
    }
}
