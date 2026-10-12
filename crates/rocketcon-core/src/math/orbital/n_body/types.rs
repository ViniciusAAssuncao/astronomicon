use astronomicon_core::units::{
    AccelerationVector, Duration, GravitationalParameter, Length, Mass, Position, Speed,
    VelocityVector,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NBodySystemBody {
    pub id: Uuid,
    pub mass: Mass,
    pub position: Position,
    pub velocity: VelocityVector,
    pub radius: Length,
    pub j2: Option<f64>,
}

impl NBodySystemBody {
    pub fn new(
        id: Uuid,
        mass: Mass,
        position: Position,
        velocity: VelocityVector,
        radius: Length,
        j2: Option<f64>,
    ) -> Self {
        Self {
            id,
            mass,
            position,
            velocity,
            radius,
            j2,
        }
    }

    pub fn gravitational_parameter(&self) -> GravitationalParameter {
        GravitationalParameter::new(
            astronomicon_core::units::constants::GRAVITATIONAL_CONSTANT * self.mass.value(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CowellPerturbationConfig {
    pub primary_body: NBodySystemBody,
    pub perturbing_bodies: Vec<NBodySystemBody>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NBodyPropagationConfig {
    pub initial_position: Position,
    pub initial_velocity: VelocityVector,
    pub initial_mass: Mass,
    pub cowell_config: CowellPerturbationConfig,
    pub start_epoch: Duration,
    pub duration: Duration,
    pub initial_step: Duration,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NBodyTrajectoryPoint {
    pub time: Duration,
    pub position: Position,
    pub velocity: VelocityVector,
    pub acceleration: AccelerationVector,
    pub specific_energy: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NBodyPropagationResult {
    pub points: Vec<NBodyTrajectoryPoint>,
    pub final_epoch: Duration,
    pub initial_specific_energy: f64,
    pub final_specific_energy: f64,
    pub energy_drift_fraction: f64,
    pub total_delta_v_absorbed: Speed,
}