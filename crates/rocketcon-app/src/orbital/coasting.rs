use crate::error::{RocketError, RocketResult};
use astronomicon_app::ephemeris::{CelestialState, EphemerisCatalog};
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::units::{Duration, GravitationalParameter, Position, VelocityVector};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::orbital::propagate_universal_state_vectors;
use rocketcon_db::repositories::vehicle_physical_state as physical_repository;
use std::collections::HashMap;
use uuid::Uuid;

fn propagate_in_moving_reference(
    position: Position,
    velocity: VelocityVector,
    reference_position: Position,
    reference_velocity: VelocityVector,
    future_reference_position: Position,
    future_reference_velocity: VelocityVector,
    mu: GravitationalParameter,
    dt: Duration,
) -> RocketResult<(Position, VelocityVector)> {
    let relative_position = Position::from_raw(position.raw() - reference_position.raw());
    let relative_velocity = VelocityVector::from_raw(velocity.raw() - reference_velocity.raw());
    let (next_position, next_velocity) =
        propagate_universal_state_vectors(relative_position, relative_velocity, mu, dt)?;
    Ok((
        Position::from_raw(next_position.raw() + future_reference_position.raw()),
        VelocityVector::from_raw(next_velocity.raw() + future_reference_velocity.raw()),
    ))
}

pub async fn propagate_coasting_vehicle(
    pool: &SqlitePool,
    physical_state: &VehiclePhysicalState,
    environment: &EnvironmentSnapshot,
    dt: Duration,
    catalog: &EphemerisCatalog,
    future_states: &HashMap<Uuid, CelestialState>,
) -> RocketResult<VehiclePhysicalState> {
    if environment.reference_id() != physical_state.reference_body_id()
        || environment.captured_universe_epoch != physical_state.captured_universe_epoch()
        || environment.captured_at_epoch != physical_state.captured_at_epoch()
    {
        return Err(RocketError::Generic(
            "coasting requires an environment matching the physical state".into(),
        ));
    }

    let current_epoch = environment.captured_total_epoch();
    let reference_id = environment.reference_id();
    let mu = gravitational_parameter(
        environment
            .planet
            .as_ref()
            .map_or(environment.star.mass(), |planet| planet.mass()),
    );
    let current = catalog
        .states(current_epoch)?
        .get(&reference_id)
        .copied()
        .ok_or_else(|| {
            RocketError::Generic("coasting reference has no current ephemeris".into())
        })?;
    let future = future_states
        .get(&reference_id)
        .copied()
        .ok_or_else(|| RocketError::Generic("coasting reference has no future ephemeris".into()))?;
    let (position, velocity) = propagate_in_moving_reference(
        physical_state.position(),
        physical_state.velocity(),
        environment.reference_position(),
        current.velocity,
        future.position,
        future.velocity,
        mu,
        dt,
    )?;
    let next = VehiclePhysicalState::new_with_max_q(
        physical_state.vehicle_id(),
        position,
        velocity,
        physical_state
            .orientation()
            .integrate(physical_state.angular_velocity(), dt),
        physical_state.angular_velocity(),
        reference_id,
        physical_state.captured_universe_epoch(),
        physical_state.captured_at_epoch() + dt,
        physical_state.max_dynamic_pressure(),
        physical_state.max_dynamic_pressure_epoch(),
    )?;
    physical_repository::upsert(pool, &next).await?;
    Ok(next)
}

pub(crate) fn propagate_coasting_segment(
    physical_state: &VehiclePhysicalState,
    catalog: &EphemerisCatalog,
    reference_id: Uuid,
    duration: Duration,
) -> RocketResult<VehiclePhysicalState> {
    let mu = catalog.mass_of_body(reference_id)
        .map(gravitational_parameter)
        .ok_or_else(|| RocketError::Generic(
            format!("reference body '{reference_id}' has no gravitational mass"),
        ))?;
    let begin_epoch = physical_state.captured_total_epoch();
    let begin = catalog.states(begin_epoch)?
        .get(&reference_id).copied()
        .ok_or_else(|| RocketError::Generic(
            format!("reference '{reference_id}' has no initial state"),
        ))?;
    let end = catalog.states(begin_epoch + duration)?
        .get(&reference_id).copied()
        .ok_or_else(|| RocketError::Generic(
            format!("reference '{reference_id}' has no final state"),
        ))?;
    let (position, velocity) = propagate_in_moving_reference(
        physical_state.position(),
        physical_state.velocity(),
        begin.position,
        begin.velocity,
        end.position,
        end.velocity,
        mu,
        duration,
    )?;
    Ok(VehiclePhysicalState::new_with_max_q(
        physical_state.vehicle_id(),
        position,
        velocity,
        physical_state.orientation().integrate(physical_state.angular_velocity(), duration),
        physical_state.angular_velocity(),
        reference_id,
        physical_state.captured_universe_epoch(),
        physical_state.captured_at_epoch() + duration,
        physical_state.max_dynamic_pressure(),
        physical_state.max_dynamic_pressure_epoch(),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_duration_preserves_world_state_for_same_reference() {
        let origin = Position::from_raw(astronomicon_core::units::Vector3::new(
            250_000_000.0,
            -70_000_000.0,
            35_000.0,
        ));
        let center_velocity =
            VelocityVector::from_raw(astronomicon_core::units::Vector3::new(0.0, 29_000.0, 0.0));
        let position = Position::from_raw(
            origin.raw() + astronomicon_core::units::Vector3::new(8_000_000.0, 0.0, 0.0),
        );
        let velocity = VelocityVector::from_raw(
            center_velocity.raw() + astronomicon_core::units::Vector3::new(0.0, 7500.0, 0.0),
        );
        let (next_position, next_velocity) = propagate_in_moving_reference(
            position,
            velocity,
            origin,
            center_velocity,
            origin,
            center_velocity,
            GravitationalParameter::new(3.986004418e14),
            Duration::new(0.0),
        )
        .unwrap();
        assert!((next_position.raw() - position.raw()).magnitude() < 1e-6);
        assert!((next_velocity.raw() - velocity.raw()).magnitude() < 1e-9);
    }

    #[test]
    fn moving_frame_matches_two_body_relative_propagation() {
        use astronomicon_core::units::Vector3;
        let origin = Position::from_raw(Vector3::new(5.0e9, 1.0e9, 0.0));
        let next_origin = Position::from_raw(Vector3::new(5.0e9 + 12_000.0, 1.0e9, 0.0));
        let origin_velocity = VelocityVector::from_raw(Vector3::new(30_000.0, 0.0, 0.0));
        let relative_position = Position::from_raw(Vector3::new(7.0e6, 0.0, 0.0));
        let relative_velocity = VelocityVector::from_raw(Vector3::new(0.0, 7_500.0, 0.0));
        let dt = Duration::new(0.4);
        let mu = GravitationalParameter::new(3.986004418e14);
        let (expected_position, expected_velocity) =
            propagate_universal_state_vectors(relative_position, relative_velocity, mu, dt)
                .unwrap();
        let (actual_position, actual_velocity) = propagate_in_moving_reference(
            Position::from_raw(origin.raw() + relative_position.raw()),
            VelocityVector::from_raw(origin_velocity.raw() + relative_velocity.raw()),
            origin,
            origin_velocity,
            next_origin,
            origin_velocity,
            mu,
            dt,
        )
        .unwrap();
        assert!(
            (actual_position.raw() - next_origin.raw() - expected_position.raw()).magnitude()
                < 1e-5
        );
        assert!(
            (actual_velocity.raw() - origin_velocity.raw() - expected_velocity.raw()).magnitude()
                < 1e-8
        );
    }
}
