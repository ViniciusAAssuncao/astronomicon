use astronomicon_core::units::{
    Angle,
    ForceVector,
    Pressure,
    TorqueVector,
    Vector3,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MachRegime {
    Subsonic,
    Transonic,
    Supersonic,
    Hypersonic,
}

pub fn mach_regime(mach: f64) -> MachRegime {
    if !mach.is_finite() || mach < 0.8 {
        MachRegime::Subsonic
    } else if mach <= 1.2 {
        MachRegime::Transonic
    } else if mach < 5.0 {
        MachRegime::Supersonic
    } else {
        MachRegime::Hypersonic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AerodynamicAngles {
    pub angle_of_attack: Angle,
    pub sideslip_angle: Angle,
    pub total_angle_of_attack: Angle,
}

impl AerodynamicAngles {
    pub fn new(
        angle_of_attack: Angle,
        sideslip_angle: Angle,
        total_angle_of_attack: Angle,
    ) -> Self {
        Self {
            angle_of_attack,
            sideslip_angle,
            total_angle_of_attack,
        }
    }

    pub fn zero() -> Self {
        Self {
            angle_of_attack: Angle::new(0.0),
            sideslip_angle: Angle::new(0.0),
            total_angle_of_attack: Angle::new(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleAerodynamics {
    pub angles: AerodynamicAngles,
    pub mach: f64,
    pub mach_regime: MachRegime,
    pub dynamic_pressure: Pressure,
    pub drag_coefficient: f64,
    pub normal_force_coefficient: f64,
    pub axial_force_body: ForceVector,
    pub normal_force_body: ForceVector,
    pub total_force_body: ForceVector,
    pub total_force_world: ForceVector,
    pub torque_body: TorqueVector,
    pub torque_world: TorqueVector,
    pub center_of_pressure: Vector3,
    pub center_of_mass: Vector3,
    pub lever_arm: Vector3,
}

impl VehicleAerodynamics {
    pub fn angles(&self) -> AerodynamicAngles {
        self.angles
    }

    pub fn mach(&self) -> f64 {
        self.mach
    }

    pub fn mach_regime(&self) -> MachRegime {
        self.mach_regime
    }

    pub fn dynamic_pressure(&self) -> Pressure {
        self.dynamic_pressure
    }

    pub fn drag_coefficient(&self) -> f64 {
        self.drag_coefficient
    }

    pub fn normal_force_coefficient(&self) -> f64 {
        self.normal_force_coefficient
    }

    pub fn axial_force_body(&self) -> ForceVector {
        self.axial_force_body
    }

    pub fn normal_force_body(&self) -> ForceVector {
        self.normal_force_body
    }

    pub fn total_force_body(&self) -> ForceVector {
        self.total_force_body
    }

    pub fn total_force_world(&self) -> ForceVector {
        self.total_force_world
    }

    pub fn torque_body(&self) -> TorqueVector {
        self.torque_body
    }

    pub fn torque_world(&self) -> TorqueVector {
        self.torque_world
    }

    pub fn center_of_pressure(&self) -> Vector3 {
        self.center_of_pressure
    }

    pub fn center_of_mass(&self) -> Vector3 {
        self.center_of_mass
    }

    pub fn lever_arm(&self) -> Vector3 {
        self.lever_arm
    }
}