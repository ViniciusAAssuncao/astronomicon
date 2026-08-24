use crate::error::DbResult;
use crate::models::BarycenterRow;
use crate::repositories::fetch::{fetch_all, fetch_all_by_param, fetch_optional_by_param};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, star_system_id, name, primary_star_id, primary_planet_id, \
    primary_barycenter_id, secondary_star_id, secondary_planet_id, secondary_barycenter_id, \
    internal_semi_major_axis_m, internal_eccentricity, internal_inclination_rad, \
    internal_longitude_ascending_node_rad, internal_argument_periapsis_rad, \
    internal_mean_anomaly_at_epoch_rad, parent_star_id, parent_planet_id, \
    parent_barycenter_id, parent_minor_planet_id, external_semi_major_axis_m, \
    external_eccentricity, external_inclination_rad, external_longitude_ascending_node_rad, \
    external_argument_periapsis_rad, external_mean_anomaly_at_epoch_rad FROM barycenters";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<BarycenterRow>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    fetch_optional_by_param(pool, &query, id.to_string()).await
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<BarycenterRow>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    fetch_all(pool, &query).await
}

pub async fn list_by_system(pool: &SqlitePool, system_id: &Uuid) -> DbResult<Vec<BarycenterRow>> {
    let query = format!("{BASE_QUERY} WHERE star_system_id = ? ORDER BY name ASC");
    fetch_all_by_param(pool, &query, system_id.to_string()).await
}
