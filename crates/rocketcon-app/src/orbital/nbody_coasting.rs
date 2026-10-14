use super::nbody_field::{CatalogGravityField, tick_policy};
use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::n_body::dop853::solve_dop853;
use rocketcon_db::repositories::vehicle_physical_state as physical_repository;

pub(crate) fn opt_in_enabled(value: Option<&str>) -> RocketResult<bool> {
    match value {
        None | Some("0") => Ok(false),
        Some("1") => Ok(true),
        Some(_) => Err(RocketError::Generic(
            "ROCKETCON_C15_NBODY_COAST accepts only 0 or 1".into(),
        )),
    }
}

pub(crate) fn coasting_nbody_requested() -> RocketResult<bool> {
    match std::env::var("ROCKETCON_C15_NBODY_COAST") {
        Ok(value) => opt_in_enabled(Some(&value)),
        Err(std::env::VarError::NotPresent) => opt_in_enabled(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(RocketError::Generic(
            "ROCKETCON_C15_NBODY_COAST is not valid Unicode".into(),
        )),
    }
}

fn integrate_coast(
    initial: &VehiclePhysicalState,
    dt: Duration,
    catalog: &EphemerisCatalog,
) -> RocketResult<(Position, VelocityVector)> {
    let origin = initial.position().raw();
    let current_velocity = initial.velocity().raw();
    let mut gravity = CatalogGravityField::new(catalog, initial.captured_total_epoch(), origin)?;
    if ![current_velocity.0, current_velocity.1, current_velocity.2]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(RocketError::Generic(
            "nonfinite initial orbital velocity".into(),
        ));
    }
    let config = tick_policy(dt, [1e-3, 1e-3, 1e-3, 1e-5, 1e-5, 1e-5])?;
    let solved = solve_dop853(
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
    )
    .map_err(|error| RocketError::Generic(format!("N-body coasting failed: {error:?}")))?;
    if solved.event_seconds.is_some() || solved.reached_seconds != dt.value() {
        return Err(RocketError::Generic(
            "incomplete N-body coasting tick".into(),
        ));
    }
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
) -> RocketResult<VehiclePhysicalState> {
    let (position, velocity) = integrate_coast(previous, dt, catalog)?;
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
    fn nbody_opt_in_is_disabled_by_default_and_rejects_unknown_values() {
        assert!(!opt_in_enabled(None).unwrap());
        assert!(!opt_in_enabled(Some("0")).unwrap());
        assert!(opt_in_enabled(Some("1")).unwrap());
        for value in ["", "true", "yes", "2", "-1"] {
            assert!(opt_in_enabled(Some(value)).is_err());
        }
    }

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
