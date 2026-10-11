use super::orbit_apsides::{next_apoapsis, next_periapsis, OrbitApsis};
use super::RocketconSession;
use crate::{RocketError, RocketResult};
use astronomicon_core::units::Duration;
use rocketcon_app::orbital::resolve_relative_state_for_body;
use rocketcon_core::math::orbital::state_properties::orbital_period_if_bound;
use rocketcon_core::math::orbital::{
    cartesian_to_osculating_elements, propagate_universal_state_vectors,
};
use rocketcon_db::repositories::vehicle_physical_state;
use uuid::Uuid;

const SAMPLE_INTERVALS: usize = 192;
const MIN_HORIZON_S: f64 = 600.0;
const MAX_HORIZON_S: f64 = 21_600.0;

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitPreview {
    pub reference_body_id: Uuid,
    pub source_epoch_seconds: f64,
    pub horizon_seconds: f64,
    pub impact_epoch_seconds: Option<f64>,
    pub under_thrust: bool,
    pub eccentricity: Option<f64>,
    pub periapsis_altitude_m: Option<f64>,
    pub apoapsis_altitude_m: Option<f64>,
    pub period_seconds: Option<f64>,
    pub next_periapsis: Option<OrbitApsis>,
    pub next_apoapsis: Option<OrbitApsis>,
    pub relative_points_m: Vec<[f64; 3]>,
}

impl RocketconSession {
    pub async fn orbit_preview(&self) -> RocketResult<OrbitPreview> {
        let state = vehicle_physical_state::get_by_vehicle_id(&self.pool, &self.vehicle_id)
            .await?
            .ok_or_else(|| RocketError::Generic("vehicle physical state is unavailable".into()))?;
        let reference_body_id = state.reference_body_id();
        let epoch = state.captured_total_epoch();
        let environment = rocketcon_app::environment::load_environment_snapshot(
            &self.pool,
            reference_body_id,
            self.universe_epoch,
            state.captured_at_epoch(),
        )
        .await?;
        let (position, velocity, mu) = resolve_relative_state_for_body(
            &self.pool,
            &state,
            reference_body_id,
            environment.system_id,
            epoch,
        )
        .await?;
        let body_radius = self
            .snapshot
            .reference_body_radius_m
            .filter(|radius| radius.is_finite() && *radius > 0.0);
        let body_radius_m = body_radius.unwrap_or(0.0);
        let elements = cartesian_to_osculating_elements(position, velocity, mu)?;
        let periapsis_altitude = elements.periapsis_distance.value() - body_radius_m;
        let periapsis_altitude_m =
            body_radius.and_then(|_| periapsis_altitude.is_finite().then_some(periapsis_altitude));
        let apoapsis_altitude_m = elements
            .apoapsis_distance
            .map(|distance| distance.value() - body_radius_m)
            .filter(|value| body_radius.is_some() && value.is_finite());
        let period_seconds = orbital_period_if_bound(&elements, mu)
            .map(|period| period.value())
            .filter(|value| value.is_finite());
        let horizon_seconds =
            period_seconds.map_or(3_600.0, |period| period.clamp(MIN_HORIZON_S, MAX_HORIZON_S));
        if !horizon_seconds.is_finite() {
            return Err(RocketError::Generic(
                "orbital preview horizon is not finite".into(),
            ));
        }
        let radius = body_radius_m;
        let mut relative_points_m = Vec::with_capacity(SAMPLE_INTERVALS + 1);
        let mut impact_epoch_seconds = None;
        let mut sample_position = position;
        let mut sample_velocity = velocity;
        for index in 0..=SAMPLE_INTERVALS {
            let elapsed = horizon_seconds * index as f64 / SAMPLE_INTERVALS as f64;
            if index > 0 {
                (sample_position, sample_velocity) = propagate_universal_state_vectors(
                    sample_position,
                    sample_velocity,
                    mu,
                    Duration::new(horizon_seconds / SAMPLE_INTERVALS as f64),
                )?;
            }
            let point = sample_position.raw();
            if ![point.0, point.1, point.2]
                .iter()
                .all(|value| value.is_finite())
            {
                return Err(RocketError::Generic(
                    "orbital preview contains non-finite coordinates".into(),
                ));
            }
            relative_points_m.push([point.0, point.1, point.2]);
            if radius > 0.0 && index > 0 && point.magnitude() <= radius {
                impact_epoch_seconds = Some(epoch.value() + elapsed);
                break;
            }
        }
        let visible_horizon = impact_epoch_seconds
            .map(|impact| impact - epoch.value())
            .unwrap_or(horizon_seconds);
        Ok(OrbitPreview {
            reference_body_id,
            source_epoch_seconds: epoch.value(),
            horizon_seconds,
            impact_epoch_seconds,
            under_thrust: self
                .snapshot
                .main_engine_loads
                .iter()
                .any(|(_, load)| *load > 0.0),
            eccentricity: elements.eccentricity.is_finite()
                .then_some(elements.eccentricity),
            periapsis_altitude_m,
            apoapsis_altitude_m,
            period_seconds,
            next_periapsis: next_periapsis(&elements, position, velocity, mu, visible_horizon),
            next_apoapsis: next_apoapsis(&elements, position, velocity, mu, visible_horizon),
            relative_points_m,
        })
    }
}
