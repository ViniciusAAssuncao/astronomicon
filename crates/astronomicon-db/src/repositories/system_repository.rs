use crate::error::DbResult;
use crate::models::StarSystemRow;
use crate::repositories::fetch::{fetch_all, fetch_optional_by_param};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str =
    "SELECT id, name, right_ascension_rad, declination_rad, distance_from_sol_m FROM star_systems";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<StarSystemRow>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    fetch_optional_by_param(pool, &query, id.to_string()).await
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<StarSystemRow>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    fetch_all(pool, &query).await
}
