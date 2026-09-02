use crate::error::RocketDbResult;
use crate::models::EnergyReservoirStateRow;
use rocketcon_core::domain::EnergyReservoirState;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_component_id, stored_energy_j, captured_universe_epoch_s, captured_at_epoch_s FROM energy_reservoir_states";

pub async fn get_by_vehicle_component_id(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<Option<EnergyReservoirState>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_component_id = ?");
    let row = sqlx::query_as::<_, EnergyReservoirStateRow>(&query)
        .bind(vehicle_component_id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(EnergyReservoirState::try_from).transpose()
}

pub async fn upsert(
    pool: &SqlitePool,
    state: &EnergyReservoirState,
) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO energy_reservoir_states (vehicle_component_id, stored_energy_j, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, ?, ?, ?) ON CONFLICT(vehicle_component_id) DO UPDATE SET stored_energy_j = excluded.stored_energy_j, captured_universe_epoch_s = excluded.captured_universe_epoch_s, captured_at_epoch_s = excluded.captured_at_epoch_s",
    )
    .bind(state.vehicle_component_id().to_string())
    .bind(state.stored_energy().value())
    .bind(state.captured_universe_epoch().value())
    .bind(state.captured_at_epoch().value())
    .execute(pool)
    .await?;

    Ok(())
}