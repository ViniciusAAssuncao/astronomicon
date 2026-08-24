use crate::error::DbResult;
use crate::models::UniverseStateRow;
use crate::repositories::fetch::fetch_all;
use sqlx::SqlitePool;

const BASE_QUERY: &str = "SELECT id, seconds_since_j2000_epoch FROM universe_state";

pub async fn get(pool: &SqlitePool) -> DbResult<Option<UniverseStateRow>> {
    let query = format!("{BASE_QUERY} WHERE id = 1");
    let row = sqlx::query_as::<_, UniverseStateRow>(&query)
        .fetch_optional(pool)
        .await?;

    Ok(row)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<UniverseStateRow>> {
    let query = format!("{BASE_QUERY} ORDER BY id ASC");
    fetch_all(pool, &query).await
}
