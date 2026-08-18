use crate::error::DbResult;
use crate::models::BarycenterRow;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<BarycenterRow>> {
    let row = sqlx::query_as::<_, BarycenterRow>(
        "SELECT id, star_system_id, name, primary_star_id, primary_planet_id, primary_barycenter_id, \
         secondary_star_id, secondary_planet_id, secondary_barycenter_id, internal_semi_major_axis_m, \
         internal_eccentricity, internal_inclination_rad, internal_longitude_ascending_node_rad, \
         internal_argument_periapsis_rad, internal_mean_anomaly_at_epoch_rad, parent_star_id, \
         parent_planet_id, parent_barycenter_id, external_semi_major_axis_m, external_eccentricity, \
         external_inclination_rad, external_longitude_ascending_node_rad, external_argument_periapsis_rad, \
         external_mean_anomaly_at_epoch_rad \
         FROM barycenters WHERE id = ?",
    )
    .bind(id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<BarycenterRow>> {
    let rows = sqlx::query_as::<_, BarycenterRow>(
        "SELECT id, star_system_id, name, primary_star_id, primary_planet_id, primary_barycenter_id, \
         secondary_star_id, secondary_planet_id, secondary_barycenter_id, internal_semi_major_axis_m, \
         internal_eccentricity, internal_inclination_rad, internal_longitude_ascending_node_rad, \
         internal_argument_periapsis_rad, internal_mean_anomaly_at_epoch_rad, parent_star_id, \
         parent_planet_id, parent_barycenter_id, external_semi_major_axis_m, external_eccentricity, \
         external_inclination_rad, external_longitude_ascending_node_rad, external_argument_periapsis_rad, \
         external_mean_anomaly_at_epoch_rad \
         FROM barycenters ORDER BY name ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn list_by_system(pool: &SqlitePool, system_id: &Uuid) -> DbResult<Vec<BarycenterRow>> {
    let rows = sqlx::query_as::<_, BarycenterRow>(
        "SELECT id, star_system_id, name, primary_star_id, primary_planet_id, primary_barycenter_id, \
         secondary_star_id, secondary_planet_id, secondary_barycenter_id, internal_semi_major_axis_m, \
         internal_eccentricity, internal_inclination_rad, internal_longitude_ascending_node_rad, \
         internal_argument_periapsis_rad, internal_mean_anomaly_at_epoch_rad, parent_star_id, \
         parent_planet_id, parent_barycenter_id, external_semi_major_axis_m, external_eccentricity, \
         external_inclination_rad, external_longitude_ascending_node_rad, external_argument_periapsis_rad, \
         external_mean_anomaly_at_epoch_rad \
         FROM barycenters WHERE star_system_id = ? ORDER BY name ASC",
    )
    .bind(system_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}