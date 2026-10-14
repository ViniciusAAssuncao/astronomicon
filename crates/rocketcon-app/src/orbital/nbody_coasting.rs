use super::nbody_field::{CatalogGravityField, tick_policy};
use super::nbody_impact::CatalogImpactMonitor;
use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::n_body::dop853::solve_dop853_with_event;
use rocketcon_db::repositories::vehicle_physical_state as physical_repository;
use uuid::Uuid;

fn integrate_coast(
    initial: &VehiclePhysicalState,
    dt: Duration,
    catalog: &EphemerisCatalog,
    selected_ids: &[Uuid],
    position_segments: &mut Vec<
        rocketcon_core::math::orbital::n_body::dop853::Dop853DenseSegment<3>,
    >,
) -> RocketResult<(Position, VelocityVector)> {
    let origin = initial.position().raw();
    let current_velocity = initial.velocity().raw();
    let mut gravity = CatalogGravityField::new(
        catalog,
        initial.captured_total_epoch(),
        origin,
        Some(selected_ids),
    )?;
    if ![current_velocity.0, current_velocity.1, current_velocity.2]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(RocketError::Generic(
            "nonfinite initial orbital velocity".into(),
        ));
    }
    let config = tick_policy(dt, [1e-3, 1e-3, 1e-3, 1e-5, 1e-5, 1e-5])?;
    let impact = CatalogImpactMonitor::new(
        catalog,
        initial.captured_total_epoch(),
        origin,
        gravity.impact_candidates(),
    );
    let solved = solve_dop853_with_event(
        [
            0.0,
            0.0,
            0.0,
            current_velocity.0,
            current_velocity.1,
            current_velocity.2,
        ],
        dt.value(),
        config,
        |t, state| {
            let acceleration = gravity
                .acceleration_at(t, Vector3::new(state[0], state[1], state[2]))?
                .raw();
            Ok([
                state[3],
                state[4],
                state[5],
                acceleration.0,
                acceleration.1,
                acceleration.2,
            ])
        },
        || false,
        |segment| Ok(impact.first_in(segment)?.map(|event| event.elapsed_seconds)),
    )
    .map_err(|error| RocketError::Generic(format!("N-body coasting failed: {error:?}")))?;
    if solved.event_seconds.is_some() || solved.reached_seconds != dt.value() {
        return Err(RocketError::Generic(
            "incomplete N-body coasting tick".into(),
        ));
    }
    let projected = solved
        .segments
        .iter()
        .map(|segment| {
            segment.position_segment().ok_or_else(|| {
                RocketError::Generic("N-body coasting dense position unavailable".into())
            })
        })
        .collect::<RocketResult<Vec<_>>>()?;
    position_segments.extend(projected);
    let y = solved.final_state;
    Ok((
        Position::from_raw(origin + Vector3::new(y[0], y[1], y[2])),
        VelocityVector::from_raw(Vector3::new(y[3], y[4], y[5])),
    ))
}

pub(crate) async fn propagate_nbody_coasting_vehicle(
    pool: &SqlitePool,
    previous: &VehiclePhysicalState,
    dt: Duration,
    catalog: &EphemerisCatalog,
    selected_ids: &[Uuid],
    position_segments: &mut Vec<
        rocketcon_core::math::orbital::n_body::dop853::Dop853DenseSegment<3>,
    >,
) -> RocketResult<VehiclePhysicalState> {
    let (position, velocity) =
        integrate_coast(previous, dt, catalog, selected_ids, position_segments)?;
    let next = VehiclePhysicalState::new_with_max_q(
        previous.vehicle_id(),
        position,
        velocity,
        previous
            .orientation()
            .integrate(previous.angular_velocity(), dt),
        previous.angular_velocity(),
        previous.reference_body_id(),
        previous.captured_universe_epoch(),
        previous.captured_at_epoch() + dt,
        previous.max_dynamic_pressure(),
        previous.max_dynamic_pressure_epoch(),
    )?;
    physical_repository::upsert(pool, &next).await?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_policy_scales_step_sizes_without_fixed_physical_dt() {
        let short = tick_policy(Duration::new(0.02), [1e-3; 6]).unwrap();
        let long = tick_policy(Duration::new(0.2), [1e-3; 6]).unwrap();
        assert_eq!(long.initial_step_s, short.initial_step_s * 10.0);
        assert!(short.maximum_evaluations < usize::MAX);
        assert!(tick_policy(Duration::new(0.0), [1e-3; 6]).is_err());
        assert!(tick_policy(Duration::new(f64::NAN), [1e-3; 6]).is_err());
    }
}
