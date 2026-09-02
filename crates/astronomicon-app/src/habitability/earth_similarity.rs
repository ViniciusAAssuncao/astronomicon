use crate::climate::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::shape::{effective_polar_radius_for_planet, planet_mean_density};
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::math::habitability::{
    calculate_earth_similarity_index, EarthSimilarityIndex,
};
use astronomicon_core::math::radiometry::escape_velocity;
use astronomicon_core::math::shape::mean_radius;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use uuid::Uuid;

pub async fn resolve_earth_similarity_index(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<EarthSimilarityIndex> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let eq_radius = planet
        .equatorial_radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "equatorial_radius".to_string(),
            reason: format!("planet '{}' has no equatorial radius", planet_id),
        })?;

    let pol_radius = effective_polar_radius_for_planet(&planet);
    let mean_r = mean_radius(eq_radius, pol_radius);
    let density = planet_mean_density(&planet);
    let mu = gravitational_parameter(planet.mass());
    let v_esc = escape_velocity(mu, eq_radius);
    let mean_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;

    Ok(calculate_earth_similarity_index(
        mean_r,
        density,
        v_esc,
        mean_temp,
    ))
}
