use super::soi_crossing::{SoiCrossing, first_crossing, interpolate_world_position};
use crate::error::{RocketError, RocketResult};
use crate::orbital::coasting::propagate_coasting_segment;
use crate::orbital::invalidate_vehicle_trajectory_patches;
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::n_body::dop853::Dop853DenseSegment;
use rocketcon_core::math::orbital::propagate_universal_state_vectors;
use rocketcon_db::repositories::vehicle_physical_state as physical_repository;

pub(super) async fn apply_confirmed_soi_transition(
    pool: &SqlitePool,
    initial: &VehiclePhysicalState,
    proposed: VehiclePhysicalState,
    catalog: &EphemerisCatalog,
    duration: Duration,
    uses_force_integration: bool,
    dense_segments: Option<&[Dop853DenseSegment<3>]>,
) -> RocketResult<(VehiclePhysicalState, Vec<SoiCrossing>)> {
    let total = duration.value();
    let initial_epoch = initial.captured_total_epoch();
    let mut elapsed = 0.0;
    let mut segment_start = *initial;
    let mut current_reference = initial.reference_body_id();
    let mut endpoint = proposed;
    let mut events = Vec::new();
    let event_capacity = catalog.planets().len().saturating_mul(2).saturating_add(1);
    let resolution = initial_epoch.value().abs() * f64::EPSILON;

    for _ in 0..event_capacity {
        let remaining = total - elapsed;
        if remaining <= resolution {
            break;
        }
        let crossing = if uses_force_integration {
            let start_position = if let Some(segments) = dense_segments {
                nbody_world_position(segments, initial.position(), elapsed)?
            } else {
                interpolate_world_position(initial, &proposed, elapsed / total, total)
            };
            first_crossing(
                catalog,
                initial_epoch + Duration::new(elapsed),
                current_reference,
                remaining,
                start_position,
                proposed.position(),
                |t, _| {
                    if let Some(segments) = dense_segments {
                        nbody_world_position(segments, initial.position(), elapsed + t)
                    } else {
                        Ok(interpolate_world_position(
                            initial,
                            &proposed,
                            (elapsed + t) / total,
                            total,
                        ))
                    }
                },
            )?
        } else {
            let reference = segment_start.reference_body_id();
            let reference_initial = catalog
                .states(segment_start.captured_total_epoch())?
                .get(&reference)
                .copied()
                .ok_or_else(|| {
                    RocketError::Generic(format!("reference '{reference}' missing from ephemeris"))
                })?;
            let mu = catalog
                .mass_of_body(reference)
                .map(gravitational_parameter)
                .ok_or_else(|| {
                    RocketError::Generic(format!(
                        "reference '{reference}' has no gravitational mass"
                    ))
                })?;
            let relative_position = Position::from_raw(
                segment_start.position().raw() - reference_initial.position.raw(),
            );
            let relative_velocity = VelocityVector::from_raw(
                segment_start.velocity().raw() - reference_initial.velocity.raw(),
            );
            first_crossing(
                catalog,
                segment_start.captured_total_epoch(),
                reference,
                remaining,
                segment_start.position(),
                endpoint.position(),
                |t, positions| {
                    let (relative, _) = propagate_universal_state_vectors(
                        relative_position,
                        relative_velocity,
                        mu,
                        Duration::new(t),
                    )?;
                    let origin = positions.get(&reference).ok_or_else(|| {
                        RocketError::Generic(format!(
                            "reference '{reference}' missing during SOI search"
                        ))
                    })?;
                    Ok(Position::from_raw(relative.raw() + origin.raw()))
                },
            )?
        };
        let Some(mut crossing) = crossing else { break };
        if crossing.elapsed_seconds <= 0.0 {
            break;
        }
        if !uses_force_integration {
            let first = propagate_coasting_segment(
                &segment_start,
                catalog,
                current_reference,
                Duration::new(crossing.elapsed_seconds),
            )?;
            segment_start = with_reference(first, crossing.new_reference_id)?;
            endpoint = propagate_coasting_segment(
                &segment_start,
                catalog,
                crossing.new_reference_id,
                Duration::new((remaining - crossing.elapsed_seconds).max(0.0)),
            )?;
        }
        current_reference = crossing.new_reference_id;
        elapsed += crossing.elapsed_seconds;
        crossing.elapsed_seconds = elapsed;
        events.push(crossing);
    }
    if events.is_empty() {
        return Ok((proposed, events));
    }
    if uses_force_integration {
        endpoint = with_reference(proposed, current_reference)?;
    }
    physical_repository::upsert(pool, &endpoint).await?;
    invalidate_vehicle_trajectory_patches(pool, endpoint.vehicle_id()).await?;
    Ok((endpoint, events))
}

fn nbody_world_position(
    segments: &[Dop853DenseSegment<3>],
    origin: Position,
    elapsed: f64,
) -> RocketResult<Position> {
    let local = segments
        .iter()
        .find_map(|segment| segment.evaluate(elapsed))
        .ok_or_else(|| {
            RocketError::Generic(format!(
                "N-body dense SOI state absent at elapsed {elapsed} seconds"
            ))
        })?;
    Ok(Position::from_raw(
        origin.raw() + Vector3::new(local[0], local[1], local[2]),
    ))
}

fn with_reference(
    state: VehiclePhysicalState,
    reference_body_id: uuid::Uuid,
) -> RocketResult<VehiclePhysicalState> {
    Ok(VehiclePhysicalState::new_with_max_q(
        state.vehicle_id(),
        state.position(),
        state.velocity(),
        state.orientation(),
        state.angular_velocity(),
        reference_body_id,
        state.captured_universe_epoch(),
        state.captured_at_epoch(),
        state.max_dynamic_pressure(),
        state.max_dynamic_pressure_epoch(),
    )?)
}