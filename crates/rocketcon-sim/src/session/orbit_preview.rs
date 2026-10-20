use super::OrbitViewBasis;
use super::RocketconSession;
use super::orbit_apsides::{OrbitApsis, next_apoapsis, next_periapsis};
use super::orbit_forecast::{ForecastArc, ForecastBody, ForecastEvent, ForecastResult};
use super::orbit_forecast_hybrid::sample_hybrid_forecast_cancelled;
use super::orbit_nodes::{orbit_nodes, orbit_nodes_in_frame};
use super::orbit_preview_horizon::preview_horizon;
use super::orbit_preview_path::{PreviewEndReason, sample_preview_path};
use super::orbit_reference_curve::closed_osculating_reference;
use super::orbit_soi::{OrbitSoiPreview, soi_radius};
use super::orbit_view_basis::orbit_view_basis;
use super::planet_equatorial_reference;
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::{OrbitalParent, Planet};
use astronomicon_core::units::{Duration, Length};
use astronomicon_db::SqlitePool;
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
    pub continuation_points_m: Vec<[f64; 3]>,
    pub forecast_target_seconds: f64,
    pub forecast_completed_seconds: f64,
    pub forecast_complete: bool,
}

fn apply_forecast(preview: &mut OrbitPreview, forecast: &ForecastResult) {
    preview.future_arcs = forecast.arcs.clone();
    preview.forecast_events = forecast.events.clone();
    preview.forecast_bodies = forecast.bodies.clone();
    preview.continuation_points_m = forecast.continuation_points_m.clone();
    preview.forecast_completed_seconds = forecast.elapsed_seconds;
    preview.forecast_complete = forecast.impact_seconds.is_some()
        || forecast.elapsed_seconds
            >= preview.forecast_target_seconds
                - (preview.forecast_target_seconds * 1e-12).max(1e-6);
    if let Some(impact) = forecast.impact_seconds {
        preview.impact_epoch_seconds = Some(preview.source_epoch_seconds + impact);
    }
}
impl RocketconSession {
    pub async fn orbit_preview(&self) -> RocketResult<OrbitPreview> {
        preview_from_pool(
            &self.pool,
            self.vehicle_id,
            self.universe_epoch,
            self.snapshot.reference_body_radius_m,
            self.snapshot
                .main_engine_loads
                .iter()
                .any(|(_, load)| *load > 0.0),
        )
        .await
    }
}

pub async fn preview_from_pool(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    radius_override: Option<f64>,
    under_thrust: bool,
) -> RocketResult<OrbitPreview> {
    preview_from_pool_cancelled(
        pool,
        vehicle_id,
        universe_epoch,
        radius_override,
        under_thrust,
        &mut || false,
        &mut |_| {},
    )
    .await
}

pub async fn preview_from_pool_cancelled(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    radius_override: Option<f64>,
    under_thrust: bool,
    cancelled: &mut impl FnMut() -> bool,
    progress: &mut impl FnMut(OrbitPreview),
) -> RocketResult<OrbitPreview> {
    if cancelled() {
        return Err(RocketError::Generic("forecast cancelled".into()));
    }
    let state = vehicle_physical_state::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or_else(|| RocketError::Generic("vehicle physical state is unavailable".into()))?;
    let reference_body_id = state.reference_body_id();
    let epoch = state.captured_total_epoch();
    let environment = rocketcon_app::environment::load_environment_snapshot(
        pool,
        reference_body_id,
        universe_epoch,
        state.captured_at_epoch(),
    )
    .await?;
    let (position, velocity, mu) = resolve_relative_state_for_body(
        pool,
        &state,
        reference_body_id,
        environment.system_id,
        epoch,
    )
    .await?;
    let body_radius = radius_override
        .or_else(|| environment.reference_radius().map(|radius| radius.value()))
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
            OrbitalParent::Star(id) if id == environment.star.id() => Some(environment.star.mass()),
            OrbitalParent::Planet(id) => planet_repository::get_by_id(pool, &id)
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
    let visible_horizon = path.elapsed_seconds;
    let relative_points_m = path.points;
    let equatorial = environment
        .planet
        .as_ref()
        .and_then(planet_equatorial_reference)
        .map(|frame| EquatorialOrbitPreview {
            nodes: orbit_nodes_in_frame(&elements, position, velocity, mu, visible_horizon, frame),
            frame,
        });
    let catalog = EphemerisCatalog::load(pool, environment.system_id).await?;
    let mut forecast_horizon = horizon_seconds.max(14.0 * 86400.0);
    if environment.planet.is_none()
        && period_seconds.is_none_or(|period| period > 365.25 * 86400.0)
    {
        forecast_horizon = forecast_horizon.max(20.0 * 365.25 * 86400.0);
    }
    if let Some(period) = period_seconds {
        if period.is_finite() && period > 0.0 && period <= 400.0 * 86400.0 {
            forecast_horizon = forecast_horizon.max(period);
        }
    }
    let mut preview = OrbitPreview {
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
        impact_epoch_seconds: path.impact_seconds.map(|elapsed| epoch.value() + elapsed),
        soi,
        under_thrust,
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
        future_arcs: Vec::new(),
        forecast_events: Vec::new(),
        forecast_bodies: Vec::new(),
        continuation_points_m: Vec::new(),
        forecast_target_seconds: forecast_horizon,
        forecast_completed_seconds: 0.0,
        forecast_complete: false,
        osculating_closed_points_m: closed_osculating_reference(
            position,
            velocity,
            mu,
            &elements,
            body_radius.map(Length::new),
        )
        .unwrap_or_default(),
    };
    let forecast = sample_hybrid_forecast_cancelled(
        &catalog,
        &state,
        forecast_horizon,
        cancelled,
        &mut |partial| {
            let mut snapshot = preview.clone();
            apply_forecast(&mut snapshot, partial);
            progress(snapshot);
        },
    )?;
    apply_forecast(&mut preview, &forecast);
    Ok(preview)
}
