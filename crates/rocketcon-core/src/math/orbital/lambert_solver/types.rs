use crate::math::orbital::types::OrbitType;
use astronomicon_core::units::{Angle, Duration, Length, Speed, VelocityVector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransferDirection {
    ShortWay,
    LongWay,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LambertSolution {
    pub departure_velocity: VelocityVector,
    pub arrival_velocity: VelocityVector,
    pub semi_major_axis: Length,
    pub eccentricity: f64,
    pub transfer_angle: Angle,
    pub orbit_type: OrbitType,
    pub time_of_flight: Duration,
    pub transfer_direction: TransferDirection,
}

impl LambertSolution {
    pub fn new(
        departure_velocity: VelocityVector,
        arrival_velocity: VelocityVector,
        semi_major_axis: Length,
        eccentricity: f64,
        transfer_angle: Angle,
        orbit_type: OrbitType,
        time_of_flight: Duration,
        transfer_direction: TransferDirection,
    ) -> Self {
        Self {
            departure_velocity,
            arrival_velocity,
            semi_major_axis,
            eccentricity,
            transfer_angle,
            orbit_type,
            time_of_flight,
            transfer_direction,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PorkchopPoint {
    pub departure_excess_speed: Speed,
    pub arrival_excess_speed: Speed,
    pub characteristic_energy_c3: f64,
    pub total_delta_v: Speed,
    pub time_of_flight: Duration,
    pub solution: LambertSolution,
}

impl PorkchopPoint {
    pub fn new(
        departure_excess_speed: Speed,
        arrival_excess_speed: Speed,
        characteristic_energy_c3: f64,
        total_delta_v: Speed,
        time_of_flight: Duration,
        solution: LambertSolution,
    ) -> Self {
        Self {
            departure_excess_speed,
            arrival_excess_speed,
            characteristic_energy_c3,
            total_delta_v,
            time_of_flight,
            solution,
        }
    }
}