use crate::error::AppResult;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::lagrange::orbital_plane_normal;
use astronomicon_core::math::rotation_state::{
    planet_rotation_angle_at_epoch, planet_rotation_axis_direction,
    resolve_planet_body_orientation, solstice_reference_direction,
};
use astronomicon_core::units::{Angle, Duration, Quaternion, Vector3};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use uuid::Uuid;

pub async fn resolve_planet_orientation(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Quaternion> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let total_epoch = universe_epoch + at_epoch;
    let rot_period = planet
        .rotation_period()
        .unwrap_or_else(|| Duration::new(86400.0));
    let pmea = planet
        .prime_meridian_epoch_angle()
        .unwrap_or_else(|| Angle::new(0.0));
    let rot_angle = planet_rotation_angle_at_epoch(rot_period, pmea, total_epoch);

    let obliquity = planet.obliquity().unwrap_or_else(|| Angle::new(0.0));
    let solstice_ta = planet
        .solstice_true_anomaly()
        .unwrap_or_else(|| Angle::new(0.0));

    let (normal, solstice_ref) = if let Some(elements) = planet.orbital_elements() {
        (
            orbital_plane_normal(&elements),
            solstice_reference_direction(&elements, solstice_ta),
        )
    } else {
        (Vector3::new(0.0, 0.0, 1.0), Vector3::new(1.0, 0.0, 0.0))
    };

    let spin_axis = planet_rotation_axis_direction(normal, obliquity, solstice_ref);
    Ok(resolve_planet_body_orientation(spin_axis, rot_angle))
}