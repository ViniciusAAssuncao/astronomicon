use super::orbit_forecast::{
    ForecastArc, ForecastBody, ForecastEvent, ForecastResult, MAX_ARCS, body_name,
};
use super::orbit_forecast_continuation::terminal_continuation;
use super::orbit_forecast_geometry::sample_smooth_curve;
use super::orbit_preview_horizon::MAX_BOUND_HORIZON;
use crate::catalog_nbody::{CatalogTrajectory, integrate_catalog_trajectory_from_offset};
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use rocketcon_app::aeroespacial::first_crossing;
use rocketcon_app::orbital::hybrid_regime::{GravityRegime, RegimePolicy, choose_regime_at_offset};
use rocketcon_core::math::orbital::n_body::dop853::{
    Dop853Config, Dop853DenseSegment, Dop853Error,
};
use uuid::Uuid;

const MAX_FORECAST_GEOMETRY_POINTS: usize = 1536;
const MAX_NUMERICAL_WINDOWS: usize = 4;

fn position_at(segment: &Dop853DenseSegment<6>, time: f64) -> RocketResult<Position> {
    let state = segment
        .evaluate(time)
        .ok_or_else(|| RocketError::Generic("forecast dense state unavailable".into()))?;
    Ok(Position::from_raw(Vector3::new(
        state[0], state[1], state[2],
    )))
}

fn point(world: Position, center: Vector3) -> [f64; 3] {
    let relative = world.raw() - center;
    [relative.0, relative.1, relative.2]
}

fn reference_center(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    first_reference: Uuid,
    elapsed_seconds: f64,
) -> RocketResult<Vector3> {
    catalog
        .positions_for_ids_at_offset(epoch, elapsed_seconds, &[first_reference])
        .map_err(|error| RocketError::Generic(error.to_string()))?
        .get(&first_reference)
        .map(|position| position.raw())
        .ok_or_else(|| RocketError::Generic("forecast display center unavailable".into()))
}

fn push_point(result: &mut ForecastResult, position: [f64; 3], seconds: f64) {
    if let Some(arc) = result.arcs.last_mut() {
        if arc.points_m.last().copied() != Some(position) {
            arc.points_m.push(position);
        }
        arc.end_seconds = seconds;
    }
    result.elapsed_seconds = seconds;
}

fn append_curve(
    result: &mut ForecastResult,
    segment: &Dop853DenseSegment<6>,
    start: f64,
    end: f64,
    offset: f64,
    first_reference: Uuid,
    catalog: &EphemerisCatalog,
    epoch: Duration,
) -> RocketResult<()> {
    if end <= start {
        return Ok(());
    }
    let used = result
        .arcs
        .iter()
        .map(|arc| arc.points_m.len())
        .sum::<usize>();
    let relative_at = |time| -> RocketResult<Position> {
        let world = position_at(segment, time)?;
        let center = reference_center(catalog, epoch, first_reference, offset + time)?;
        Ok(Position::from_raw(world.raw() - center))
    };
    let sampled = sample_smooth_curve(
        start,
        end,
        relative_at(start)?,
        relative_at(end)?,
        MAX_FORECAST_GEOMETRY_POINTS.saturating_sub(used),
        relative_at,
    ).map_err(|error| RocketError::Generic(format!(
        "forecast geometry at segment [{start:.3}, {end:.3}], offset {offset:.3}, previously used {used} points: {error}"
    )))?;
    for (time, relative) in sampled {
        let position = relative.raw();
        push_point(result, [position.0, position.1, position.2], offset + time);
    }
    Ok(())
}

fn window_policy(seconds: f64) -> Dop853Config<6> {
    Dop853Config {
        absolute_tolerances: [0.01, 0.01, 0.01, 1e-5, 1e-5, 1e-5],
        relative_tolerance: 1e-9,
        initial_step_s: seconds / 96.0,
        minimum_step_s: (seconds * 1e-11).max(1e-8),
        maximum_step_s: seconds / 48.0,
        maximum_attempts: 384,
        maximum_evaluations: 5800,
        maximum_segments: 256,
    }
}

fn integrate_window(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    offset: f64,
    position: Position,
    velocity: VelocityVector,
    requested: f64,
    source_ids: &[Uuid],
    cancelled: &mut impl FnMut() -> bool,
) -> RocketResult<CatalogTrajectory> {
    let mut seconds = requested;
    let minimum = seconds / 16.0;
    loop {
        if cancelled() {
            return Err(RocketError::Generic("forecast cancelled".into()));
        }
        let candidate = integrate_catalog_trajectory_from_offset(
            catalog,
            epoch,
            offset,
            position,
            velocity,
            Duration::new(seconds),
            Some(source_ids),
            window_policy(seconds),
            &mut *cancelled,
        );
        match candidate {
            Ok(result) => return Ok(result),
            Err(Dop853Error::Cancelled) => {
                return Err(RocketError::Generic("forecast cancelled".into()));
            }
            Err(Dop853Error::BudgetExceeded | Dop853Error::StepUnderflow) if seconds > minimum => {
                seconds *= 0.5;
            }
            Err(error) => {
                return Err(RocketError::Generic(format!(
                    "restricted forecast window failed: {error:?}"
                )));
            }
        }
    }
}

fn append_window(
    result: &mut ForecastResult,
    catalog: &EphemerisCatalog,
    epoch: Duration,
    window_offset: f64,
    first_reference: Uuid,
    reference: &mut Uuid,
    trajectory: &CatalogTrajectory,
) -> RocketResult<()> {
    for segment in &trajectory.integrated.segments {
        let start = segment.start_seconds;
        let end = segment.valid_until_seconds;
        if end <= start {
            continue;
        }
        let crossing = first_crossing(
            catalog,
            epoch + Duration::new(window_offset + start),
            *reference,
            end - start,
            position_at(segment, start)?,
            position_at(segment, end)?,
            |local, _| position_at(segment, start + local),
        )?;
        let mut continued_from = start;
        if let Some(crossing) = crossing {
            let crossing_local = start + crossing.elapsed_seconds;
            let crossing_absolute = window_offset + crossing_local;
            append_curve(
                result,
                segment,
                start,
                crossing_local,
                window_offset,
                first_reference,
                catalog,
                epoch,
            )?;
            continued_from = crossing_local;
            let display_center = reference_center(
                catalog, epoch, first_reference, crossing_absolute,
            )?;
            let display_point = point(position_at(segment, crossing_local)?, display_center);
            result.events.push(ForecastEvent {
                kind: if crossing.entering { 0 } else { 1 },
                body_name: body_name(
                    catalog,
                    if crossing.entering {
                        crossing.new_reference_id
                    } else {
                        *reference
                    },
                ),
                seconds: crossing_absolute,
                position_m: display_point,
            });
            if result.arcs.len() >= MAX_ARCS {
                return Err(RocketError::Generic(
                    "forecast SOI arc budget exhausted".into(),
                ));
            }
            *reference = crossing.new_reference_id;
            if let Some(radius_m) = catalog.radius_of_body(*reference) {
                let positions = catalog
                    .positions_at_offset(epoch, crossing_absolute)
                    .map_err(|error| RocketError::Generic(error.to_string()))?;
                let center = positions.get(reference).ok_or_else(|| {
                    RocketError::Generic("forecast SOI body position unavailable".into())
                })?;
                result.bodies.push(ForecastBody {
                    name: body_name(catalog, *reference),
                    radius_m,
                    position_m: point(*center, display_center),
                });
            }
            result.arcs.push(ForecastArc {
                reference_body_id: *reference,
                start_seconds: crossing_absolute,
                end_seconds: crossing_absolute,
                points_m: vec![display_point],
            });
        }
        append_curve(
            result,
            segment,
            continued_from,
            end,
            window_offset,
            first_reference,
            catalog,
            epoch,
        )?;
    }
    if let Some(impact) = trajectory.impact {
        let y = trajectory.integrated.final_state;
        let world = Position::from_raw(Vector3::new(y[0], y[1], y[2]));
        let when = window_offset + impact.elapsed_seconds;
        let center = reference_center(catalog, epoch, first_reference, when)?;
        result.impact_seconds = Some(when);
        result.events.push(ForecastEvent {
            kind: 2,
            body_name: body_name(catalog, impact.body_id),
            seconds: when,
            position_m: point(world, center),
        });
    }
    Ok(())
}

pub(super) fn sample_restricted_forecast(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    first_reference: Uuid,
    position: Position,
    velocity: VelocityVector,
    horizon: f64,
    initial_sources: &[Uuid],
    cancelled: &mut impl FnMut() -> bool,
    progress: &mut impl FnMut(&ForecastResult),
) -> RocketResult<ForecastResult> {
    let states = catalog.states(epoch)?;
    let initial_center = states
        .get(&first_reference)
        .ok_or_else(|| RocketError::Generic("forecast reference has no ephemeris".into()))?
        .position
        .raw();
    let mut result = ForecastResult {
        arcs: vec![ForecastArc {
            reference_body_id: first_reference,
            start_seconds: 0.0,
            end_seconds: 0.0,
            points_m: vec![point(position, initial_center)],
        }],
        events: Vec::new(),
        bodies: Vec::new(),
        elapsed_seconds: 0.0,
        impact_seconds: None,
        continuation_points_m: Vec::new(),
    };
    if let Some(radius_m) = catalog.radius_of_body(first_reference) {
        result.bodies.push(ForecastBody {
            name: body_name(catalog, first_reference),
            radius_m,
            position_m: [0.0; 3],
        });
    }
    if !horizon.is_finite() || horizon <= 0.0 {
        return Ok(result);
    }
    let mut offset = 0.0;
    let mut current_position = position;
    let mut current_velocity = velocity;
    let mut reference = first_reference;
    let mut sources = initial_sources.to_vec();
    for window in 0..MAX_NUMERICAL_WINDOWS {
        if cancelled() {
            return Err(RocketError::Generic("forecast cancelled".into()));
        }
        if offset >= horizon {
            break;
        }
        let requested = (horizon - offset).min(MAX_BOUND_HORIZON);
        if window > 0 {
            let decision = choose_regime_at_offset(
                catalog,
                epoch,
                offset,
                reference,
                current_position.raw(),
                current_velocity.raw(),
                Duration::new(requested),
                false,
                RegimePolicy::forecast(),
            )
            .map_err(|error| RocketError::Generic(error.to_string()))?;
            if let GravityRegime::Restricted {
                sources: additional,
            } = decision
            {
                sources.extend(additional);
                sources.sort();
                sources.dedup();
            }
        }
        let trajectory = integrate_window(
            catalog,
            epoch,
            offset,
            current_position,
            current_velocity,
            requested,
            &sources,
            cancelled,
        )?;
        if cancelled() {
            return Err(RocketError::Generic("forecast cancelled".into()));
        }
        append_window(
            &mut result,
            catalog,
            epoch,
            offset,
            first_reference,
            &mut reference,
            &trajectory,
        )?;
        if result.impact_seconds.is_some() {
            if !cancelled() {
                progress(&result);
            }
            break;
        }
        let y = trajectory.integrated.final_state;
        let expected = point(
            Position::from_raw(Vector3::new(y[0], y[1], y[2])),
            reference_center(catalog, epoch, first_reference, offset + trajectory.integrated.reached_seconds)?,
        );
        let actual = result
            .arcs
            .last()
            .and_then(|arc| arc.points_m.last())
            .ok_or_else(|| RocketError::Generic("forecast window has no endpoint".into()))?;
        let residual = expected
            .iter()
            .zip(actual)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        if !residual.is_finite() || residual > 1.0 {
            return Err(RocketError::Generic(format!(
                "forecast dense/window endpoint discontinuity: {residual} m"
            )));
        }
        let reached = trajectory.integrated.reached_seconds;
        if reached <= 0.0 || !reached.is_finite() {
            return Err(RocketError::Generic(
                "forecast numerical window did not advance".into(),
            ));
        }
        current_position = Position::from_raw(Vector3::new(y[0], y[1], y[2]));
        current_velocity = VelocityVector::from_raw(Vector3::new(y[3], y[4], y[5]));
        offset += reached;
        if cancelled() {
            return Err(RocketError::Generic("forecast cancelled".into()));
        }
        progress(&result);
    }
    if offset > 0.0 && result.impact_seconds.is_none() {
        result.continuation_points_m = terminal_continuation(
            catalog, epoch, first_reference, reference, offset,
            current_position, current_velocity,
        ).unwrap_or_default();
        if let (Some(first), Some(last)) = (
            result.continuation_points_m.first_mut(),
            result.arcs.last().and_then(|arc| arc.points_m.last()),
        ) {
            *first = *last;
        }
    }
    Ok(result)
}
