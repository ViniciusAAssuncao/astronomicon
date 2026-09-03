use crate::math::orbital::types::OsculatingElements;
use astronomicon_core::units::{
    AngularVelocity, Duration, Length, Position, Speed, VelocityVector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GaussVariationalRates {
    pub semi_major_axis_rate: Speed,
    pub eccentricity_rate: f64,
    pub inclination_rate: AngularVelocity,
    pub raan_rate: AngularVelocity,
    pub argument_of_periapsis_rate: AngularVelocity,
    pub true_anomaly_rate: AngularVelocity,
}

impl GaussVariationalRates {
    pub fn new(
        semi_major_axis_rate: Speed,
        eccentricity_rate: f64,
        inclination_rate: AngularVelocity,
        raan_rate: AngularVelocity,
        argument_of_periapsis_rate: AngularVelocity,
        true_anomaly_rate: AngularVelocity,
    ) -> Self {
        Self {
            semi_major_axis_rate,
            eccentricity_rate,
            inclination_rate,
            raan_rate,
            argument_of_periapsis_rate,
            true_anomaly_rate,
        }
    }

    pub fn zero() -> Self {
        Self {
            semi_major_axis_rate: Speed::new(0.0),
            eccentricity_rate: 0.0,
            inclination_rate: AngularVelocity::new(0.0),
            raan_rate: AngularVelocity::new(0.0),
            argument_of_periapsis_rate: AngularVelocity::new(0.0),
            true_anomaly_rate: AngularVelocity::new(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ZonalHarmonics {
    pub j2: f64,
    pub j3: f64,
    pub j4: f64,
}

impl ZonalHarmonics {
    pub fn new(j2: f64, j3: f64, j4: f64) -> Self {
        Self { j2, j3, j4 }
    }

    pub fn j2_only(j2: f64) -> Self {
        Self {
            j2,
            j3: 0.0,
            j4: 0.0,
        }
    }

    pub fn zero() -> Self {
        Self {
            j2: 0.0,
            j3: 0.0,
            j4: 0.0,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.j2 == 0.0 && self.j3 == 0.0 && self.j4 == 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SecularDecayRates {
    pub semi_major_axis_rate: Speed,
    pub eccentricity_rate: f64,
    pub nodal_precession_rate: AngularVelocity,
    pub apsidal_precession_rate: AngularVelocity,
    pub mean_motion_correction: AngularVelocity,
}

impl SecularDecayRates {
    pub fn new(
        semi_major_axis_rate: Speed,
        eccentricity_rate: f64,
        nodal_precession_rate: AngularVelocity,
        apsidal_precession_rate: AngularVelocity,
        mean_motion_correction: AngularVelocity,
    ) -> Self {
        Self {
            semi_major_axis_rate,
            eccentricity_rate,
            nodal_precession_rate,
            apsidal_precession_rate,
            mean_motion_correction,
        }
    }

    pub fn zero() -> Self {
        Self {
            semi_major_axis_rate: Speed::new(0.0),
            eccentricity_rate: 0.0,
            nodal_precession_rate: AngularVelocity::new(0.0),
            apsidal_precession_rate: AngularVelocity::new(0.0),
            mean_motion_correction: AngularVelocity::new(0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecularOrbitDecayPrediction {
    pub initial_elements: OsculatingElements,
    pub final_elements: OsculatingElements,
    pub final_position: Position,
    pub final_velocity: VelocityVector,
    pub duration_evaluated: Duration,
    pub estimated_remaining_lifetime: Option<Duration>,
    pub is_deorbited: bool,
    pub deorbit_epoch_offset: Option<Duration>,
    pub total_altitude_loss: Length,
}

impl SecularOrbitDecayPrediction {
    pub fn new(
        initial_elements: OsculatingElements,
        final_elements: OsculatingElements,
        final_position: Position,
        final_velocity: VelocityVector,
        duration_evaluated: Duration,
        estimated_remaining_lifetime: Option<Duration>,
        is_deorbited: bool,
        deorbit_epoch_offset: Option<Duration>,
        total_altitude_loss: Length,
    ) -> Self {
        Self {
            initial_elements,
            final_elements,
            final_position,
            final_velocity,
            duration_evaluated,
            estimated_remaining_lifetime,
            is_deorbited,
            deorbit_epoch_offset,
            total_altitude_loss,
        }
    }
}
