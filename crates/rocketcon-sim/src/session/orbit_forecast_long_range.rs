use super::orbit_forecast::{ForecastResult, MAX_ARCS, sample_patched_forecast_in_frame};
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Position, VelocityVector};
use uuid::Uuid;

pub(super) fn extend_with_patched_conics(
    result: &mut ForecastResult,
    catalog: &EphemerisCatalog,
    epoch: Duration,
    initial_reference: Uuid,
    active_reference: Uuid,
    offset: f64,
    position: Position,
    velocity: VelocityVector,
    horizon: f64,
) -> RocketResult<()> {
    let remaining = horizon - offset;
    if remaining <= 0.0 {
        return Ok(());
    }
    let extended = sample_patched_forecast_in_frame(
        catalog,
        epoch + Duration::new(offset),
        active_reference,
        initial_reference,
        position,
        velocity,
        remaining,
    )?;
    if result.arcs.len() + extended.arcs.len() > MAX_ARCS + 1 {
        return Err(RocketError::Generic("long-range SOI arc budget exhausted".into()));
    }
    for (index, mut arc) in extended.arcs.into_iter().enumerate() {
        arc.start_seconds += offset;
        arc.end_seconds += offset;
        if index == 0
            && result
                .arcs
                .last()
                .is_some_and(|previous| previous.reference_body_id == arc.reference_body_id)
        {
            if let Some(last) = result.arcs.last_mut() {
                if last.points_m.last() == arc.points_m.first() {
                    arc.points_m.remove(0);
                }
                last.points_m.extend(arc.points_m);
                last.end_seconds = arc.end_seconds;
            }
        } else {
            result.arcs.push(arc);
        }
    }
    result.events.extend(extended.events.into_iter().map(|mut event| {
        event.seconds += offset;
        event
    }));
    result.bodies.extend(extended.bodies);
    result.elapsed_seconds = offset + extended.elapsed_seconds;
    result.impact_seconds = extended.impact_seconds.map(|t| offset + t);
    result.continuation_points_m = extended.continuation_points_m;
    Ok(())
}
