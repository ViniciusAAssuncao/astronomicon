use crate::error::DbResult;
use crate::models::StarRow;
use crate::repositories::fetch::{fetch_all, fetch_all_by_param, fetch_optional_by_param};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, star_system_id, parent_star_id, parent_planet_id, \
    parent_barycenter_id, parent_minor_planet_id, name, kind, mass_kg, radius_m, \
    effective_temperature_k, rotation_period_s, axial_tilt_rad, semi_major_axis_m, \
    eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, \
    mean_anomaly_at_epoch_rad, oblateness_j2, metallicity FROM stars";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<StarRow>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    fetch_optional_by_param(pool, &query, id.to_string()).await
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<StarRow>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    fetch_all(pool, &query).await
}

pub async fn list_by_system(pool: &SqlitePool, system_id: &Uuid) -> DbResult<Vec<StarRow>> {
    let query = format!("{BASE_QUERY} WHERE star_system_id = ? ORDER BY name ASC");
    fetch_all_by_param(pool, &query, system_id.to_string()).await
}
