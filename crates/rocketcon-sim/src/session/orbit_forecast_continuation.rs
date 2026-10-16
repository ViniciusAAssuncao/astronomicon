use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::units::{Duration, Position, VelocityVector};
use rocketcon_core::math::orbital::state_properties::orbital_period_if_bound;
use rocketcon_core::math::orbital::{
    cartesian_to_osculating_elements, propagate_universal_state_vectors,
};
use uuid::Uuid;

const CONTINUATION_SEGMENTS: usize = 128;
const MAX_CONTINUATION_SECONDS: f64 = 400.0 * 86400.0;

pub(super) fn terminal_continuation(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    initial_reference: Uuid,
    terminal_reference: Uuid,
    elapsed: f64,
    position: Position,
    velocity: VelocityVector,
) -> RocketResult<Vec<[f64; 3]>> {
    let terminal_states = catalog.states(epoch + Duration::new(elapsed))?;
    let state = terminal_states
        .get(&terminal_reference)
        .ok_or_else(|| RocketError::Generic("forecast terminal reference unavailable".into()))?;
    let Some(mu) = catalog
        .mass_of_body(terminal_reference)
        .map(gravitational_parameter)
    else {
        return Ok(Vec::new());
    };
    let relative_position = Position::from_raw(position.raw() - state.position.raw());
    let relative_velocity = VelocityVector::from_raw(velocity.raw() - state.velocity.raw());
    let Ok(elements) = cartesian_to_osculating_elements(relative_position, relative_velocity, mu)
    else {
        return Ok(Vec::new());
    };
    let Some(period) = orbital_period_if_bound(&elements, mu) else {
        return Ok(Vec::new());
    };
    let period_seconds = period.value();
    if !period_seconds.is_finite()
        || period_seconds <= 0.0
        || period_seconds > MAX_CONTINUATION_SECONDS
        || catalog
            .radius_of_body(terminal_reference)
            .is_none_or(|radius| elements.periapsis_distance.value() <= radius)
    {
        return Ok(Vec::new());
    }
    let mut points = Vec::with_capacity(CONTINUATION_SEGMENTS + 1);
    for index in 0..=CONTINUATION_SEGMENTS {
        let seconds = period_seconds * index as f64 / CONTINUATION_SEGMENTS as f64;
        let (relative, _) = match propagate_universal_state_vectors(
            relative_position,
            relative_velocity,
            mu,
            Duration::new(seconds),
        ) {
            Ok(value) => value,
            Err(_) => return Ok(Vec::new()),
        };
        let ids = [terminal_reference, initial_reference];
        let requested = if terminal_reference == initial_reference {
            &ids[..1]
        } else {
            &ids[..]
        };
        let centers = catalog
            .positions_for_ids_at_offset(epoch, elapsed + seconds, requested)
            .map_err(|error| RocketError::Generic(error.to_string()))?;
        let center = centers
            .get(&terminal_reference)
            .ok_or_else(|| RocketError::Generic("terminal center unavailable".into()))?;
        let origin = centers
            .get(&initial_reference)
            .ok_or_else(|| RocketError::Generic("display center unavailable".into()))?;
        let point = center.raw() + relative.raw() - origin.raw();
        if ![point.0, point.1, point.2]
            .iter()
            .all(|component| component.is_finite())
        {
            return Ok(Vec::new());
        }
        points.push([point.0, point.1, point.2]);
    }
    Ok(points)
}
