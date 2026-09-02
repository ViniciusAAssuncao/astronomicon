use crate::error::RocketDbResult;
use crate::models::ComponentOperationalStateRow;
use rocketcon_core::domain::ComponentOperationalState;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_component_id, load_fraction, current_gimbal_pitch_rad, current_gimbal_yaw_rad, captured_universe_epoch_s, captured_at_epoch_s FROM component_operational_states";

pub async fn get_by_vehicle_component_id(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<Option<ComponentOperationalState>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_component_id = ?");
    let row = sqlx::query_as::<_, ComponentOperationalStateRow>(&query)
        .bind(vehicle_component_id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(ComponentOperationalState::try_from).transpose()
}

pub async fn upsert(
    pool: &SqlitePool,
    state: &ComponentOperationalState,
) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO component_operational_states (
            vehicle_component_id,
            load_fraction,
            current_gimbal_pitch_rad,
            current_gimbal_yaw_rad,
            captured_universe_epoch_s,
            captured_at_epoch_s
        ) VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(vehicle_component_id) DO UPDATE SET
            load_fraction = excluded.load_fraction,
            current_gimbal_pitch_rad = excluded.current_gimbal_pitch_rad,
            current_gimbal_yaw_rad = excluded.current_gimbal_yaw_rad,
            captured_universe_epoch_s = excluded.captured_universe_epoch_s,
            captured_at_epoch_s = excluded.captured_at_epoch_s",
    )
    .bind(state.vehicle_component_id().to_string())
    .bind(state.load_fraction())
    .bind(state.current_gimbal_pitch().map(|a| a.value()))
    .bind(state.current_gimbal_yaw().map(|a| a.value()))
    .bind(state.captured_universe_epoch().value())
    .bind(state.captured_at_epoch().value())
    .execute(pool)
    .await?;

    Ok(())
}
