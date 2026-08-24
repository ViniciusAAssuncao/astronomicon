use crate::domain::validation::{
    validate_finite, validate_half_open_unit_interval, validate_positive_finite,
};
use crate::error::DomainResult;
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
        validate_positive_finite(semi_major_axis.value(), "semi_major_axis")?;
        validate_half_open_unit_interval(eccentricity, "eccentricity")?;
        validate_finite(inclination.value(), "inclination")?;
        validate_finite(
            longitude_of_ascending_node.value(),
            "longitude_of_ascending_node",
        )?;
        validate_finite(argument_of_periapsis.value(), "argument_of_periapsis")?;
        validate_finite(mean_anomaly_at_epoch.value(), "mean_anomaly_at_epoch")?;

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
        Angle::new(
            (self.longitude_of_ascending_node.value() + self.argument_of_periapsis.value())
                .rem_euclid(TAU),
        )
    }
}
