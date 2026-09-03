use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::low_thrust::evaluate_chebyshev_series;
use crate::math::orbital::universal::propagate_universal_state_vectors;
use crate::math::orbital::{osculating_elements_to_cartesian, OrbitType, OsculatingElements};
use astronomicon_core::domain::validation::{
    validate_finite, validate_non_negative_finite, validate_positive_finite,
};
use astronomicon_core::units::{
    Angle, Duration, Force, GravitationalParameter, Length, Mass, Position, Speed, VelocityVector,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConicPatchData {
    pub semi_major_axis: Length,
    pub eccentricity: f64,
    pub inclination: Angle,
    pub longitude_of_ascending_node: Angle,
    pub argument_of_periapsis: Angle,
    pub true_anomaly_at_epoch: Angle,
}

impl ConicPatchData {
    pub fn new(
        semi_major_axis: Length,
        eccentricity: f64,
        inclination: Angle,
        longitude_of_ascending_node: Angle,
        argument_of_periapsis: Angle,
        true_anomaly_at_epoch: Angle,
    ) -> RocketDomainResult<Self> {
        validate_non_negative_finite(eccentricity, "eccentricity")?;
        validate_finite(inclination.value(), "inclination")?;
        validate_finite(
            longitude_of_ascending_node.value(),
            "longitude_of_ascending_node",
        )?;
        validate_finite(argument_of_periapsis.value(), "argument_of_periapsis")?;
        validate_finite(true_anomaly_at_epoch.value(), "true_anomaly_at_epoch")?;

        if (1.0 - eccentricity).abs() >= 1e-6 {
            if !semi_major_axis.value().is_finite() || semi_major_axis.value() == 0.0 {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "semi_major_axis".to_string(),
                    reason: "must be non-zero and finite for non-parabolic orbit".to_string(),
                });
            }
        }

        Ok(Self {
            semi_major_axis,
            eccentricity,
            inclination,
            longitude_of_ascending_node,
            argument_of_periapsis,
            true_anomaly_at_epoch,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowThrustPatchData {
    pub initial_mass: Mass,
    pub final_mass: Mass,
    pub thrust: Force,
    pub specific_impulse: Duration,
    pub total_delta_v: Speed,
    pub chebyshev_x: Vec<f64>,
    pub chebyshev_y: Vec<f64>,
    pub chebyshev_z: Vec<f64>,
    pub chebyshev_vx: Vec<f64>,
    pub chebyshev_vy: Vec<f64>,
    pub chebyshev_vz: Vec<f64>,
    pub chebyshev_mass: Vec<f64>,
}

impl LowThrustPatchData {
    pub fn new(
        initial_mass: Mass,
        final_mass: Mass,
        thrust: Force,
        specific_impulse: Duration,
        total_delta_v: Speed,
        chebyshev_x: Vec<f64>,
        chebyshev_y: Vec<f64>,
        chebyshev_z: Vec<f64>,
        chebyshev_vx: Vec<f64>,
        chebyshev_vy: Vec<f64>,
        chebyshev_vz: Vec<f64>,
        chebyshev_mass: Vec<f64>,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(initial_mass.value(), "initial_mass")?;
        validate_positive_finite(final_mass.value(), "final_mass")?;
        validate_positive_finite(thrust.value(), "thrust")?;
        validate_positive_finite(specific_impulse.value(), "specific_impulse")?;
        validate_non_negative_finite(total_delta_v.value(), "total_delta_v")?;

        if chebyshev_x.is_empty()
            || chebyshev_y.len() != chebyshev_x.len()
            || chebyshev_z.len() != chebyshev_x.len()
            || chebyshev_vx.len() != chebyshev_x.len()
            || chebyshev_vy.len() != chebyshev_x.len()
            || chebyshev_vz.len() != chebyshev_x.len()
            || chebyshev_mass.len() != chebyshev_x.len()
        {
            return Err(RocketDomainError::InvalidInvariant {
                field: "chebyshev_coefficients".to_string(),
                reason: "coefficient vector lengths must match and be non-empty".to_string(),
            });
        }

        Ok(Self {
            initial_mass,
            final_mass,
            thrust,
            specific_impulse,
            total_delta_v,
            chebyshev_x,
            chebyshev_y,
            chebyshev_z,
            chebyshev_vx,
            chebyshev_vy,
            chebyshev_vz,
            chebyshev_mass,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrajectoryPatchKind {
    Conic(ConicPatchData),
    LowThrust(LowThrustPatchData),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryPatch {
    id: Uuid,
    vehicle_id: Uuid,
    reference_body_id: Uuid,
    start_universe_epoch: Duration,
    end_universe_epoch: Option<Duration>,
    gravitational_parameter: GravitationalParameter,
    kind: TrajectoryPatchKind,
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
        let conic = ConicPatchData::new(
            semi_major_axis,
            eccentricity,
            inclination,
            longitude_of_ascending_node,
            argument_of_periapsis,
            true_anomaly_at_epoch,
        )?;

        Self::new_with_kind(
            id,
            vehicle_id,
            reference_body_id,
            start_universe_epoch,
            end_universe_epoch,
            gravitational_parameter,
            TrajectoryPatchKind::Conic(conic),
        )
    }

    pub fn new_low_thrust(
        id: Uuid,
        vehicle_id: Uuid,
        reference_body_id: Uuid,
        start_universe_epoch: Duration,
        end_universe_epoch: Duration,
        gravitational_parameter: GravitationalParameter,
        low_thrust_data: LowThrustPatchData,
    ) -> RocketDomainResult<Self> {
        Self::new_with_kind(
            id,
            vehicle_id,
            reference_body_id,
            start_universe_epoch,
            Some(end_universe_epoch),
            gravitational_parameter,
            TrajectoryPatchKind::LowThrust(low_thrust_data),
        )
    }

    pub fn new_with_kind(
        id: Uuid,
        vehicle_id: Uuid,
        reference_body_id: Uuid,
        start_universe_epoch: Duration,
        end_universe_epoch: Option<Duration>,
        gravitational_parameter: GravitationalParameter,
        kind: TrajectoryPatchKind,
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

        validate_positive_finite(
            gravitational_parameter.value(),
            "gravitational_parameter",
        )?;

        if matches!(kind, TrajectoryPatchKind::LowThrust(_)) && end_universe_epoch.is_none() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "end_universe_epoch".to_string(),
                reason: "powered low-thrust patch must have a finite end_universe_epoch".to_string(),
            });
        }

        Ok(Self {
            id,
            vehicle_id,
            reference_body_id,
            start_universe_epoch,
            end_universe_epoch,
            gravitational_parameter,
            kind,
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

    pub fn from_low_thrust_patch_data(
        id: Uuid,
        vehicle_id: Uuid,
        reference_body_id: Uuid,
        start_universe_epoch: Duration,
        end_universe_epoch: Duration,
        gravitational_parameter: GravitationalParameter,
        low_thrust_data: LowThrustPatchData,
    ) -> RocketDomainResult<Self> {
        Self::new_low_thrust(
            id,
            vehicle_id,
            reference_body_id,
            start_universe_epoch,
            end_universe_epoch,
            gravitational_parameter,
            low_thrust_data,
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

    pub fn gravitational_parameter(&self) -> GravitationalParameter {
        self.gravitational_parameter
    }

    pub fn kind(&self) -> &TrajectoryPatchKind {
        &self.kind
    }

    pub fn is_conic(&self) -> bool {
        matches!(self.kind, TrajectoryPatchKind::Conic(_))
    }

    pub fn is_powered(&self) -> bool {
        matches!(self.kind, TrajectoryPatchKind::LowThrust(_))
    }

    pub fn conic_data(&self) -> Option<&ConicPatchData> {
        match &self.kind {
            TrajectoryPatchKind::Conic(data) => Some(data),
            TrajectoryPatchKind::LowThrust(_) => None,
        }
    }

    pub fn low_thrust_data(&self) -> Option<&LowThrustPatchData> {
        match &self.kind {
            TrajectoryPatchKind::LowThrust(data) => Some(data),
            TrajectoryPatchKind::Conic(_) => None,
        }
    }

    pub fn semi_major_axis(&self) -> Option<Length> {
        self.conic_data().map(|c| c.semi_major_axis)
    }

    pub fn eccentricity(&self) -> Option<f64> {
        self.conic_data().map(|c| c.eccentricity)
    }

    pub fn inclination(&self) -> Option<Angle> {
        self.conic_data().map(|c| c.inclination)
    }

    pub fn longitude_of_ascending_node(&self) -> Option<Angle> {
        self.conic_data().map(|c| c.longitude_of_ascending_node)
    }

    pub fn argument_of_periapsis(&self) -> Option<Angle> {
        self.conic_data().map(|c| c.argument_of_periapsis)
    }

    pub fn true_anomaly_at_epoch(&self) -> Option<Angle> {
        self.conic_data().map(|c| c.true_anomaly_at_epoch)
    }

    pub fn total_delta_v(&self) -> Option<Speed> {
        self.low_thrust_data().map(|lt| lt.total_delta_v)
    }

    pub fn propellant_consumed(&self) -> Option<Mass> {
        self.low_thrust_data()
            .map(|lt| Mass::new((lt.initial_mass.value() - lt.final_mass.value()).max(0.0)))
    }

    pub fn is_active_at(&self, epoch: Duration) -> bool {
        let t = epoch.value();
        if t < self.start_universe_epoch.value() {
            return false;
        }
        match self.end_universe_epoch {
            Some(end) => t <= end.value(),
            None => true,
        }
    }

    pub fn evaluate_state_at(
        &self,
        epoch: Duration,
    ) -> RocketDomainResult<(Position, VelocityVector)> {
        match &self.kind {
            TrajectoryPatchKind::Conic(conic) => {
                let elements = OsculatingElements::new(
                    conic.semi_major_axis,
                    conic.eccentricity,
                    conic.inclination,
                    conic.longitude_of_ascending_node,
                    conic.argument_of_periapsis,
                    conic.true_anomaly_at_epoch,
                    Length::new(0.0),
                    None,
                    0.0,
                    astronomicon_core::units::Vector3::zero(),
                    if conic.eccentricity < 1e-6 {
                        OrbitType::Circular
                    } else if conic.eccentricity < 1.0 {
                        OrbitType::Elliptic
                    } else if (1.0 - conic.eccentricity).abs() < 1e-6 {
                        OrbitType::Parabolic
                    } else {
                        OrbitType::Hyperbolic
                    },
                );

                let (r0, v0) =
                    osculating_elements_to_cartesian(&elements, self.gravitational_parameter)?;
                let dt = epoch - self.start_universe_epoch;
                propagate_universal_state_vectors(r0, v0, self.gravitational_parameter, dt)
            }
            TrajectoryPatchKind::LowThrust(lt) => {
                let t_end = self.end_universe_epoch.unwrap_or(self.start_universe_epoch);
                let t0 = self.start_universe_epoch.value();
                let t1 = t_end.value();
                let dt = t1 - t0;
                let tau = if dt > 1e-9 {
                    ((2.0 * (epoch.value() - t0) / dt) - 1.0).clamp(-1.0, 1.0)
                } else {
                    0.0
                };

                let x = evaluate_chebyshev_series(&lt.chebyshev_x, tau);
                let y = evaluate_chebyshev_series(&lt.chebyshev_y, tau);
                let z = evaluate_chebyshev_series(&lt.chebyshev_z, tau);

                let vx = evaluate_chebyshev_series(&lt.chebyshev_vx, tau);
                let vy = evaluate_chebyshev_series(&lt.chebyshev_vy, tau);
                let vz = evaluate_chebyshev_series(&lt.chebyshev_vz, tau);

                Ok((
                    Position::from_components(x, y, z),
                    VelocityVector::from_components(vx, vy, vz),
                ))
            }
        }
    }

    pub fn evaluate_mass_at(&self, epoch: Duration) -> Option<Mass> {
        match &self.kind {
            TrajectoryPatchKind::Conic(_) => None,
            TrajectoryPatchKind::LowThrust(lt) => {
                let t_end = self.end_universe_epoch.unwrap_or(self.start_universe_epoch);
                let t0 = self.start_universe_epoch.value();
                let t1 = t_end.value();
                let dt = t1 - t0;
                let tau = if dt > 1e-9 {
                    ((2.0 * (epoch.value() - t0) / dt) - 1.0).clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                let m = evaluate_chebyshev_series(&lt.chebyshev_mass, tau);
                Some(Mass::new(m.max(0.0)))
            }
        }
    }
}