use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::universal::propagate_universal_state_vectors;
use crate::math::orbital::{osculating_elements_to_cartesian, OsculatingElements};
use astronomicon_core::domain::validation::{
    validate_finite, validate_non_negative_finite, validate_positive_finite,
};
use astronomicon_core::units::{
    Angle, Duration, GravitationalParameter, Length, Position, VelocityVector,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryPatch {
    id: Uuid,
    vehicle_id: Uuid,
    reference_body_id: Uuid,
    start_universe_epoch: Duration,
    end_universe_epoch: Option<Duration>,
    semi_major_axis: Length,
    eccentricity: f64,
    inclination: Angle,
    longitude_of_ascending_node: Angle,
    argument_of_periapsis: Angle,
    true_anomaly_at_epoch: Angle,
    gravitational_parameter: GravitationalParameter,
}

impl TrajectoryPatch {
    pub fn new(
        id: Uuid,
        vehicle_id: Uuid,
        reference_body_id: Uuid,
        start_universe_epoch: Duration,
        end_universe_epoch: Option<Duration>,
        semi_major_axis: Length,
        eccentricity: f64,
        inclination: Angle,
        longitude_of_ascending_node: Angle,
        argument_of_periapsis: Angle,
        true_anomaly_at_epoch: Angle,
        gravitational_parameter: GravitationalParameter,
    ) -> RocketDomainResult<Self> {
        if id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        if vehicle_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "vehicle_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }
        if reference_body_id.is_nil() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "reference_body_id".to_string(),
                reason: "cannot be nil".to_string(),
            });
        }

        validate_finite(start_universe_epoch.value(), "start_universe_epoch")?;
        if let Some(end_epoch) = end_universe_epoch {
            validate_finite(end_epoch.value(), "end_universe_epoch")?;
            if end_epoch.value() <= start_universe_epoch.value() {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "end_universe_epoch".to_string(),
                    reason: "must be strictly greater than start_universe_epoch".to_string(),
                });
            }
        }

        validate_non_negative_finite(eccentricity, "eccentricity")?;
        validate_finite(inclination.value(), "inclination")?;
        validate_finite(
            longitude_of_ascending_node.value(),
            "longitude_of_ascending_node",
        )?;
        validate_finite(argument_of_periapsis.value(), "argument_of_periapsis")?;
        validate_finite(true_anomaly_at_epoch.value(), "true_anomaly_at_epoch")?;
        validate_positive_finite(
            gravitational_parameter.value(),
            "gravitational_parameter",
        )?;

        if (1.0 - eccentricity).abs() >= 1e-6 {
            if !semi_major_axis.value().is_finite() || semi_major_axis.value() == 0.0 {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "semi_major_axis".to_string(),
                    reason: "must be non-zero and finite for non-parabolic orbit".to_string(),
                });
            }
        }

        Ok(Self {
            id,
            vehicle_id,
            reference_body_id,
            start_universe_epoch,
            end_universe_epoch,
            semi_major_axis,
            eccentricity,
            inclination,
            longitude_of_ascending_node,
            argument_of_periapsis,
            true_anomaly_at_epoch,
            gravitational_parameter,
        })
    }

    pub fn from_osculating_elements(
        id: Uuid,
        vehicle_id: Uuid,
        reference_body_id: Uuid,
        start_universe_epoch: Duration,
        end_universe_epoch: Option<Duration>,
        elements: &OsculatingElements,
        gravitational_parameter: GravitationalParameter,
    ) -> RocketDomainResult<Self> {
        Self::new(
            id,
            vehicle_id,
            reference_body_id,
            start_universe_epoch,
            end_universe_epoch,
            elements.semi_major_axis(),
            elements.eccentricity(),
            elements.inclination(),
            elements.longitude_of_ascending_node(),
            elements.argument_of_periapsis(),
            elements.true_anomaly(),
            gravitational_parameter,
        )
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn vehicle_id(&self) -> Uuid {
        self.vehicle_id
    }

    pub fn reference_body_id(&self) -> Uuid {
        self.reference_body_id
    }

    pub fn start_universe_epoch(&self) -> Duration {
        self.start_universe_epoch
    }

    pub fn end_universe_epoch(&self) -> Option<Duration> {
        self.end_universe_epoch
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

    pub fn true_anomaly_at_epoch(&self) -> Angle {
        self.true_anomaly_at_epoch
    }

    pub fn gravitational_parameter(&self) -> GravitationalParameter {
        self.gravitational_parameter
    }

    pub fn is_active_at(&self, epoch: Duration) -> bool {
        let t = epoch.value();
        if t < self.start_universe_epoch.value() {
            return false;
        }
        match self.end_universe_epoch {
            Some(end) => t < end.value(),
            None => true,
        }
    }

    pub fn evaluate_state_at(
        &self,
        epoch: Duration,
    ) -> RocketDomainResult<(Position, VelocityVector)> {
        let elements = OsculatingElements::new(
            self.semi_major_axis,
            self.eccentricity,
            self.inclination,
            self.longitude_of_ascending_node,
            self.argument_of_periapsis,
            self.true_anomaly_at_epoch,
            Length::new(0.0),
            None,
            0.0,
            astronomicon_core::units::Vector3::zero(),
            if self.eccentricity < 1e-6 {
                crate::math::orbital::OrbitType::Circular
            } else if self.eccentricity < 1.0 {
                crate::math::orbital::OrbitType::Elliptic
            } else if (1.0 - self.eccentricity).abs() < 1e-6 {
                crate::math::orbital::OrbitType::Parabolic
            } else {
                crate::math::orbital::OrbitType::Hyperbolic
            },
        );

        let (r0, v0) = osculating_elements_to_cartesian(&elements, self.gravitational_parameter)?;
        let dt = epoch - self.start_universe_epoch;
        propagate_universal_state_vectors(r0, v0, self.gravitational_parameter, dt)
    }
}