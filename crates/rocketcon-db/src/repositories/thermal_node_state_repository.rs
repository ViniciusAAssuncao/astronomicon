use crate::error::RocketDbResult;
use crate::models::ThermalNodeStateRow;
use rocketcon_core::domain::ThermalNodeState;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_component_id, current_temperature_k, captured_universe_epoch_s, captured_at_epoch_s FROM thermal_node_states";

pub async fn get_by_vehicle_component_id(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<Option<ThermalNodeState>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_component_id = ?");
    let row = sqlx::query_as::<_, ThermalNodeStateRow>(&query)
        .bind(vehicle_component_id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(ThermalNodeState::try_from).transpose()
}

pub async fn list_for_vehicle(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
) -> RocketDbResult<Vec<ThermalNodeState>> {
    let query = "SELECT t.vehicle_component_id, t.current_temperature_k, t.captured_universe_epoch_s, t.captured_at_epoch_s FROM thermal_node_states t INNER JOIN vehicle_components vc ON vc.id = t.vehicle_component_id WHERE vc.vehicle_id = ?";
    let rows = sqlx::query_as::<_, ThermalNodeStateRow>(query)
        .bind(vehicle_id.to_string())
        .fetch_all(pool)
        .await?;

    rows.into_iter().map(ThermalNodeState::try_from).collect()
}

pub async fn upsert(
    pool: &SqlitePool,
    state: &ThermalNodeState,
) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO thermal_node_states (
            vehicle_component_id,
            current_temperature_k,
            captured_universe_epoch_s,
            captured_at_epoch_s
        ) VALUES (?, ?, ?, ?)
        ON CONFLICT(vehicle_component_id) DO UPDATE SET
            current_temperature_k = excluded.current_temperature_k,
            captured_universe_epoch_s = excluded.captured_universe_epoch_s,
            captured_at_epoch_s = excluded.captured_at_epoch_s",
    )
    .bind(state.vehicle_component_id().to_string())
    .bind(state.current_temperature_k())
    .bind(state.captured_universe_epoch().value())
    .bind(state.captured_at_epoch().value())
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<()> {
    sqlx::query("DELETE FROM thermal_node_states WHERE vehicle_component_id = ?")
        .bind(vehicle_component_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}
