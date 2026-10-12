use crate::error::RocketResult;
use astronomicon_app::ephemeris::resolve_planet_orientation;
use astronomicon_app::shape::effective_polar_radius_for_planet;
use astronomicon_core::math::rotation::angular_velocity_from_rotation_period;
use astronomicon_core::units::{
    AngularVelocityVector, Duration, Length, Position, Quaternion, Vector3, VelocityVector,
};
use astronomicon_db::SqlitePool;
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::collision::{SurfaceContactState, resolve_surface_contact};

pub(super) async fn contact_for_reference(
    pool: &SqlitePool,
    environment: &EnvironmentSnapshot,
    universe_epoch: Duration,
    at_epoch: Duration,
    body_position: Position,
    position: Position,
    velocity: VelocityVector,
) -> RocketResult<SurfaceContactState> {
    let radius = environment
        .reference_radius()
        .unwrap_or_else(|| Length::new(0.0));
    if radius.value() <= 0.0 {
        return Ok(SurfaceContactState::new(
            false,
            Length::new(0.0),
            Vector3::zero(),
            VelocityVector::zero(),
        ));
    }
    let (polar_radius, orientation, omega) = match environment.planet.as_ref() {
        Some(planet) => {
            let orientation =
                resolve_planet_orientation(pool, planet.id(), universe_epoch, at_epoch).await?;
            let spin = orientation.rotate_vector(Vector3::new(0.0, 0.0, 1.0));
            let rotation = planet
                .rotation_period()
                .unwrap_or_else(|| Duration::new(86400.0));
            (
                effective_polar_radius_for_planet(planet),
                orientation,
                AngularVelocityVector::from_raw(
                    spin * angular_velocity_from_rotation_period(rotation).value(),
                ),
            )
        }
        None => (
            radius,
            Quaternion::identity(),
            AngularVelocityVector::from_raw(Vector3::zero()),
        ),
    };
    Ok(resolve_surface_contact(
        radius,
        polar_radius,
        body_position,
        orientation,
        omega,
        position,
        velocity,
    ))
}
