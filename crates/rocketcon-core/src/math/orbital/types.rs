use astronomicon_core::units::{Angle, Length, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrbitType {
    Circular,
    Elliptic,
    Parabolic,
    Hyperbolic,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OsculatingElements {
    pub semi_major_axis: Length,
    pub eccentricity: f64,
    pub inclination: Angle,
    pub longitude_of_ascending_node: Angle,
    pub argument_of_periapsis: Angle,
    pub true_anomaly: Angle,
    pub periapsis_distance: Length,
    pub apoapsis_distance: Option<Length>,
    pub specific_orbital_energy: f64,
    pub specific_angular_momentum: Vector3,
    pub orbit_type: OrbitType,
}

impl OsculatingElements {
    pub fn new(
        semi_major_axis: Length,
        eccentricity: f64,
        inclination: Angle,
        longitude_of_ascending_node: Angle,
        argument_of_periapsis: Angle,
        true_anomaly: Angle,
        periapsis_distance: Length,
        apoapsis_distance: Option<Length>,
        specific_orbital_energy: f64,
        specific_angular_momentum: Vector3,
        orbit_type: OrbitType,
    ) -> Self {
        Self {
            semi_major_axis,
            eccentricity,
            inclination,
            longitude_of_ascending_node,
            argument_of_periapsis,
            true_anomaly,
            periapsis_distance,
            apoapsis_distance,
            specific_orbital_energy,
            specific_angular_momentum,
            orbit_type,
        }
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

    pub fn true_anomaly(&self) -> Angle {
        self.true_anomaly
    }

    pub fn periapsis_distance(&self) -> Length {
        self.periapsis_distance
    }

    pub fn apoapsis_distance(&self) -> Option<Length> {
        self.apoapsis_distance
    }

    pub fn specific_orbital_energy(&self) -> f64 {
        self.specific_orbital_energy
    }

    pub fn specific_angular_momentum(&self) -> Vector3 {
        self.specific_angular_momentum
    }

    pub fn orbit_type(&self) -> OrbitType {
        self.orbit_type
    }

    pub fn is_bound(&self) -> bool {
        self.eccentricity < 1.0
    }

    pub fn is_escape(&self) -> bool {
        self.eccentricity >= 1.0
    }
}