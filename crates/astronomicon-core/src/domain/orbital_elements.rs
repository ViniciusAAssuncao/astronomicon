use crate::error::{DomainError, DomainResult};
use crate::units::{Angle, Length};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrbitalElements {
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    longitude_of_ascending_node: Angle,
    argument_of_periapsis: Angle,
    mean_anomaly_at_epoch: Angle,
}

impl OrbitalElements {
    pub fn new(
        semi_major_axis: Length,
        eccentricity: f64,
        inclination: Angle,
        longitude_of_ascending_node: Angle,
        argument_of_periapsis: Angle,
        mean_anomaly_at_epoch: Angle,
    ) -> DomainResult<Self> {
        if !semi_major_axis.value().is_finite() || semi_major_axis.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "semi_major_axis".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !eccentricity.is_finite() || eccentricity < 0.0 || eccentricity >= 1.0 {
            return Err(DomainError::InvalidInvariant {
                field: "eccentricity".to_string(),
                reason: "must be in range [0, 1)".to_string(),
            });
        }

        if !inclination.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "inclination".to_string(),
                reason: "must be finite".to_string(),
            });
        }

        if !longitude_of_ascending_node.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "longitude_of_ascending_node".to_string(),
                reason: "must be finite".to_string(),
            });
        }

        if !argument_of_periapsis.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "argument_of_periapsis".to_string(),
                reason: "must be finite".to_string(),
            });
        }

        if !mean_anomaly_at_epoch.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "mean_anomaly_at_epoch".to_string(),
                reason: "must be finite".to_string(),
            });
        }

        let normalize = |angle: Angle| Angle::new(angle.value().rem_euclid(TAU));

        Ok(Self {
            semi_major_axis,
            eccentricity,
            inclination: normalize(inclination),
            longitude_of_ascending_node: normalize(longitude_of_ascending_node),
            argument_of_periapsis: normalize(argument_of_periapsis),
            mean_anomaly_at_epoch: normalize(mean_anomaly_at_epoch),
        })
    }

    pub fn semi_major_axis(&self) -> Length {
        self.semi_major_axis
    }

    pub fn eccentricity(&self) -> f64 {
        self.eccentricity
    }

    pub fn inclination(&self) -> Angle {
        self.inclination
    }

    pub fn longitude_of_ascending_node(&self) -> Angle {
        self.longitude_of_ascending_node
    }

    pub fn argument_of_periapsis(&self) -> Angle {
        self.argument_of_periapsis
    }

    pub fn mean_anomaly_at_epoch(&self) -> Angle {
        self.mean_anomaly_at_epoch
    }

    pub fn longitude_of_periapsis(&self) -> Angle {
        Angle::new((self.longitude_of_ascending_node.value() + self.argument_of_periapsis.value()).rem_euclid(TAU))
    }
}