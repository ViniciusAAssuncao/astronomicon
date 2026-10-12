use crate::error::RocketDbResult;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn count(pool: &SqlitePool, component_id: Uuid) -> RocketDbResult<Option<i64>> {
    let result = sqlx::query_scalar(
        "SELECT ignition_count FROM engine_ignition_history WHERE vehicle_component_id = ?",
    )
    .bind(component_id.to_string())
    .fetch_optional(pool)
    .await?;
    Ok(result)
}

pub async fn record_ignition(pool: &SqlitePool, component_id: Uuid, count: i64) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO engine_ignition_history (vehicle_component_id, ignition_count) VALUES (?, ?)
         ON CONFLICT(vehicle_component_id) DO UPDATE SET ignition_count = excluded.ignition_count",
    )
    .bind(component_id.to_string())
    .bind(count)
    .execute(pool)
    .await?;
    Ok(())
}
