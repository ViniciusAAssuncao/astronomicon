use crate::error::RocketDbResult;
use crate::models::ComponentPayloadStateRow;
use rocketcon_core::domain::ComponentPayloadState;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_component_id, is_deployed, captured_universe_epoch_s, captured_at_epoch_s FROM component_payload_states";

pub async fn get_by_vehicle_component_id(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<Option<ComponentPayloadState>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_component_id = ?");
    let row = sqlx::query_as::<_, ComponentPayloadStateRow>(&query)
        .bind(vehicle_component_id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(ComponentPayloadState::try_from).transpose()
}

pub async fn upsert(
    pool: &SqlitePool,
    state: &ComponentPayloadState,
) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO component_payload_states (vehicle_component_id, is_deployed, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, ?, ?, ?) ON CONFLICT(vehicle_component_id) DO UPDATE SET is_deployed = excluded.is_deployed, captured_universe_epoch_s = excluded.captured_universe_epoch_s, captured_at_epoch_s = excluded.captured_at_epoch_s",
    )
    .bind(state.vehicle_component_id().to_string())
    .bind(if state.is_deployed() { 1i64 } else { 0i64 })
    .bind(state.captured_universe_epoch().value())
    .bind(state.captured_at_epoch().value())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn is_deployed(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<Option<bool>> {
    let state = get_by_vehicle_component_id(pool, vehicle_component_id).await?;
    Ok(state.map(|s| s.is_deployed()))
}