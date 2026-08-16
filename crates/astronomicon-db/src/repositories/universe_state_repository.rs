use crate::error::DbResult;
use crate::models::UniverseStateRow;
use sqlx::SqlitePool;

pub async fn get(pool: &SqlitePool) -> DbResult<Option<UniverseStateRow>> {
    let row = sqlx::query_as::<_, UniverseStateRow>(
        "SELECT id, seconds_since_j2000_epoch FROM universe_state WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<UniverseStateRow>> {
    let rows = sqlx::query_as::<_, UniverseStateRow>(
        "SELECT id, seconds_since_j2000_epoch FROM universe_state ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
