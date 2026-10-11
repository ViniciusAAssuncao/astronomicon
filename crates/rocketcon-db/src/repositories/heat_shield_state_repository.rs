use crate::error::RocketDbResult;
use crate::models::HeatShieldStateRow;
use rocketcon_core::domain::HeatShieldState;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_component_id, remaining_thickness_m, surface_temperature_k, captured_universe_epoch_s, captured_at_epoch_s FROM component_heat_shield_states";

pub async fn get_by_vehicle_component_id(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<Option<HeatShieldState>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_component_id = ?");
    let row = sqlx::query_as::<_, HeatShieldStateRow>(&query)
        .bind(vehicle_component_id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(HeatShieldState::try_from).transpose()
}

pub async fn upsert(
    pool: &SqlitePool,
    state: &HeatShieldState,
) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO component_heat_shield_states (
            vehicle_component_id,
            remaining_thickness_m,
            surface_temperature_k,
            captured_universe_epoch_s,
            captured_at_epoch_s
        ) VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(vehicle_component_id) DO UPDATE SET
            remaining_thickness_m = excluded.remaining_thickness_m,
            surface_temperature_k = excluded.surface_temperature_k,
            captured_universe_epoch_s = excluded.captured_universe_epoch_s,
            captured_at_epoch_s = excluded.captured_at_epoch_s",
    )
    .bind(state.vehicle_component_id().to_string())
    .bind(state.remaining_thickness_m())
    .bind(state.surface_temperature_k())
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
    sqlx::query("DELETE FROM component_heat_shield_states WHERE vehicle_component_id = ?")
        .bind(vehicle_component_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}