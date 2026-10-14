use super::OrbitViewBasis;
use super::RocketconSession;
use super::orbit_apsides::{OrbitApsis, next_apoapsis, next_periapsis};
use super::orbit_forecast::{ForecastArc, ForecastBody, ForecastEvent, sample_patched_forecast};
use super::orbit_nodes::{orbit_nodes, orbit_nodes_in_frame};
use super::orbit_preview_horizon::{MAX_BOUND_HORIZON, preview_horizon};
use super::orbit_preview_path::{PreviewEndReason, sample_preview_path};
use super::orbit_reference_curve::closed_osculating_reference;
use super::orbit_soi::{OrbitSoiPreview, soi_radius};
use super::orbit_view_basis::orbit_view_basis;
use super::planet_equatorial_reference;
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::{OrbitalParent, Planet};
use astronomicon_core::units::Length;
use astronomicon_db::repositories::planet_repository;
use rocketcon_app::orbital::resolve_relative_state_for_body;
use rocketcon_core::math::orbital::state_properties::orbital_period_if_bound;
use rocketcon_core::math::orbital::{OrbitType, cartesian_to_osculating_elements};
use rocketcon_db::repositories::vehicle_physical_state;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct EquatorialOrbitPreview {
    pub frame: super::OrbitalReferenceFrame,
    pub nodes: super::OrbitNodes,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitPreview {
    pub reference_body_id: Uuid,
    pub reference_body_name: String,
    pub orbit_classification: u8,
    pub stellar_reference: bool,
    pub source_epoch_seconds: f64,
    pub horizon_seconds: f64,
    pub rendered_horizon_seconds: f64,
    pub end_reason: PreviewEndReason,
    pub impact_epoch_seconds: Option<f64>,
    pub soi: Option<OrbitSoiPreview>,
    pub under_thrust: bool,
    pub orbit_view_basis: Option<OrbitViewBasis>,
    pub nodes: super::OrbitNodes,
    pub equatorial: Option<EquatorialOrbitPreview>,
    pub eccentricity: Option<f64>,
    pub periapsis_altitude_m: Option<f64>,
    pub apoapsis_altitude_m: Option<f64>,
    pub period_seconds: Option<f64>,
    pub next_periapsis: Option<OrbitApsis>,
    pub next_apoapsis: Option<OrbitApsis>,
    pub relative_points_m: Vec<[f64; 3]>,
    pub osculating_closed_points_m: Vec<[f64; 3]>,
    pub future_arcs: Vec<ForecastArc>,
    pub forecast_events: Vec<ForecastEvent>,
    pub forecast_bodies: Vec<ForecastBody>,
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
        let parent_mass = if let Some(planet) = environment.planet.as_ref() {
            match planet.orbital_parent() {
                OrbitalParent::Star(id) if id == environment.star.id() => {
                    Some(environment.star.mass())
                }
                OrbitalParent::Planet(id) => planet_repository::get_by_id(&self.pool, &id)
                    .await?
                    .map(Planet::try_from)
                    .transpose()?
                    .map(|parent| parent.mass()),
                _ => None,
            }
        } else {
            None
        };
        let mut soi = environment
            .planet
            .as_ref()
            .zip(parent_mass)
            .and_then(|(planet, mass)| soi_radius(planet, mass))
            .map(|radius_m| OrbitSoiPreview {
                radius_m,
                outside: position.raw().magnitude() >= radius_m,
                exit: None,
            });
        let horizon_seconds = preview_horizon(
            &elements,
            mu,
            period_seconds,
            soi.as_ref().map(|s| s.radius_m),
        );
        let path = sample_preview_path(
            position,
            velocity,
            mu,
            body_radius_m,
            horizon_seconds,
            &mut soi,
        );
        let impact_epoch_seconds = path.impact_seconds.map(|elapsed| epoch.value() + elapsed);
        let visible_horizon = path.elapsed_seconds;
        let relative_points_m = path.points;
        let equatorial = environment
            .planet
            .as_ref()
            .and_then(planet_equatorial_reference)
            .map(|frame| EquatorialOrbitPreview {
                nodes: orbit_nodes_in_frame(
                    &elements,
                    position,
                    velocity,
                    mu,
                    visible_horizon,
                    frame,
                ),
                frame,
            });
        let catalog = EphemerisCatalog::load(&self.pool, environment.system_id).await?;
        let mut forecast_horizon = horizon_seconds.max(MAX_BOUND_HORIZON);
        if let Some(period) = period_seconds {
            if period.is_finite() && period > 0.0 && period <= 400.0 * 86400.0 {
                forecast_horizon = forecast_horizon.max(period);
            }
        }
        let forecast = sample_patched_forecast(
            &catalog, epoch, reference_body_id, state.position(), state.velocity(),
            forecast_horizon,
        )?;
        let forecast_impact = forecast.impact_seconds
            .map(|elapsed| epoch.value() + elapsed);
        let impact_epoch_seconds = forecast_impact.or(impact_epoch_seconds);
        let forecast_events = forecast.events;
        let forecast_bodies = forecast.bodies;
        let future_arcs = forecast.arcs;
        Ok(OrbitPreview {
            reference_body_id,
            reference_body_name: environment.planet.as_ref().map_or_else(
                || environment.star.name().to_owned(),
                |body| body.name().to_owned(),
            ),
            orbit_classification: match elements.orbit_type() {
                OrbitType::Circular => 0,
                OrbitType::Elliptic => 1,
                OrbitType::Parabolic => 2,
                OrbitType::Hyperbolic => 3,
            },
            stellar_reference: environment.planet.is_none(),
            source_epoch_seconds: epoch.value(),
            horizon_seconds,
            rendered_horizon_seconds: visible_horizon,
            end_reason: path.end_reason,
            impact_epoch_seconds,
            soi,
            under_thrust: self
                .snapshot
                .main_engine_loads
                .iter()
                .any(|(_, load)| *load > 0.0),
            orbit_view_basis: orbit_view_basis(position, velocity),
            nodes: orbit_nodes(&elements, position, velocity, mu, visible_horizon),
            equatorial,
            eccentricity: elements
                .eccentricity
                .is_finite()
                .then_some(elements.eccentricity),
            periapsis_altitude_m,
            apoapsis_altitude_m,
            period_seconds,
            next_periapsis: next_periapsis(&elements, position, velocity, mu, visible_horizon),
            next_apoapsis: next_apoapsis(&elements, position, velocity, mu, visible_horizon),
            relative_points_m,
            future_arcs,
            forecast_events,
            forecast_bodies,
            osculating_closed_points_m: closed_osculating_reference(
                position, velocity, mu, &elements, body_radius.map(Length::new),
            ).unwrap_or_default(),
        })
    }
}
