use astronomicon_core::units::{
    Duration, Force, GravitationalParameter, Length, Mass, MassRate, Position, Speed, Vector3,
    VelocityVector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LowThrustSteeringMode {
    Tangential,
    Inertial(Vector3),
    CircularityOptimal,
    OptimalPrimerVector(Vector3),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LowThrustTrajectoryState {
    pub time: Duration,
    pub position: Position,
    pub velocity: VelocityVector,
    pub mass: Mass,
    pub accumulated_delta_v: Speed,
}

impl LowThrustTrajectoryState {
    pub fn new(
        time: Duration,
        position: Position,
        velocity: VelocityVector,
        mass: Mass,
        accumulated_delta_v: Speed,
    ) -> Self {
        Self {
            time,
            position,
            velocity,
            mass,
            accumulated_delta_v,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowThrustPropagationConfig {
    pub initial_position: Position,
    pub initial_velocity: VelocityVector,
    pub initial_mass: Mass,
    pub thrust: Force,
    pub specific_impulse: Duration,
    pub mu: GravitationalParameter,
    pub steering_mode: LowThrustSteeringMode,
    pub duration: Duration,
    pub time_step: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowThrustPropagationResult {
    pub states: Vec<LowThrustTrajectoryState>,
    pub final_mass: Mass,
    pub total_delta_v: Speed,
    pub total_propellant_consumed: Mass,
    pub flight_time: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdelbaumTransferResult {
    pub total_delta_v: Speed,
    pub flight_time: Duration,
    pub initial_mass: Mass,
    pub final_mass: Mass,
    pub propellant_consumed: Mass,
    pub mass_flow_rate: MassRate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpiralEscapeResult {
    pub initial_radius: Length,
    pub target_radius: Length,
    pub total_delta_v: Speed,
    pub flight_time: Duration,
    pub final_mass: Mass,
    pub propellant_consumed: Mass,
    pub revolutions_estimate: f64,
}