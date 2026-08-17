use crate::error::DbResult;
use crate::models::StarRow;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<StarRow>> {
    let row = sqlx::query_as::<_, StarRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, \
         name, kind, mass_kg, radius_m, effective_temperature_k, rotation_period_s, axial_tilt_rad, \
         semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2 \
         FROM stars WHERE id = ?",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<StarRow>> {
    let rows = sqlx::query_as::<_, StarRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, \
         name, kind, mass_kg, radius_m, effective_temperature_k, rotation_period_s, axial_tilt_rad, \
         semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2 \
         FROM stars ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_system(pool: &SqlitePool, system_id: &Uuid) -> DbResult<Vec<StarRow>> {
    let rows = sqlx::query_as::<_, StarRow>(
        "SELECT id, star_system_id, parent_star_id, parent_planet_id, parent_barycenter_id, \
         name, kind, mass_kg, radius_m, effective_temperature_k, rotation_period_s, axial_tilt_rad, \
         semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, \
         argument_periapsis_rad, mean_anomaly_at_epoch_rad, oblateness_j2 \
         FROM stars WHERE star_system_id = ? ORDER BY name ASC",
    )
    .bind(system_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}