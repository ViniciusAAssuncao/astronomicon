use crate::error::RocketDbResult;
use crate::models::ReactionWheelStateRow;
use rocketcon_core::domain::ReactionWheelState;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_component_id, stored_angular_momentum_n_m_s, captured_universe_epoch_s, captured_at_epoch_s FROM component_reaction_wheel_states";

pub async fn get_by_vehicle_component_id(
    pool: &SqlitePool,
    vehicle_component_id: &Uuid,
) -> RocketDbResult<Option<ReactionWheelState>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_component_id = ?");
    let row = sqlx::query_as::<_, ReactionWheelStateRow>(&query)
        .bind(vehicle_component_id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(ReactionWheelState::try_from).transpose()
}

pub async fn upsert(
    pool: &SqlitePool,
    state: &ReactionWheelState,
) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO component_reaction_wheel_states (
            vehicle_component_id,
            stored_angular_momentum_n_m_s,
            captured_universe_epoch_s,
            captured_at_epoch_s
        ) VALUES (?, ?, ?, ?)
        ON CONFLICT(vehicle_component_id) DO UPDATE SET
            stored_angular_momentum_n_m_s = excluded.stored_angular_momentum_n_m_s,
            captured_universe_epoch_s = excluded.captured_universe_epoch_s,
            captured_at_epoch_s = excluded.captured_at_epoch_s",
    )
    .bind(state.vehicle_component_id().to_string())
    .bind(state.stored_angular_momentum().value())
    .bind(state.captured_universe_epoch().value())
    .bind(state.captured_at_epoch().value())
    .execute(pool)
    .await?;

    Ok(())
}
