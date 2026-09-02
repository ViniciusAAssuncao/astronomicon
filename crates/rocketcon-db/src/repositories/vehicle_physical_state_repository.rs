use crate::error::RocketDbResult;
use crate::models::VehiclePhysicalStateRow;
use rocketcon_core::domain::VehiclePhysicalState;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_id, position_x_m, position_y_m, position_z_m, velocity_x_m_s, velocity_y_m_s, velocity_z_m_s, orientation_q_w, orientation_q_x, orientation_q_y, orientation_q_z, angular_velocity_x_rad_s, angular_velocity_y_rad_s, angular_velocity_z_rad_s, reference_body_id, captured_universe_epoch_s, captured_at_epoch_s FROM vehicle_physical_states";

pub async fn get_by_vehicle_id(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
) -> RocketDbResult<Option<VehiclePhysicalState>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_id = ?");
    let row = sqlx::query_as::<_, VehiclePhysicalStateRow>(&query)
        .bind(vehicle_id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(VehiclePhysicalState::try_from).transpose()
}

pub async fn upsert(
    pool: &SqlitePool,
    state: &VehiclePhysicalState,
) -> RocketDbResult<()> {
    sqlx::query(
        "INSERT INTO vehicle_physical_states (
            vehicle_id,
            position_x_m, position_y_m, position_z_m,
            velocity_x_m_s, velocity_y_m_s, velocity_z_m_s,
            orientation_q_w, orientation_q_x, orientation_q_y, orientation_q_z,
            angular_velocity_x_rad_s, angular_velocity_y_rad_s, angular_velocity_z_rad_s,
            reference_body_id,
            captured_universe_epoch_s, captured_at_epoch_s
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(vehicle_id) DO UPDATE SET
            position_x_m = excluded.position_x_m,
            position_y_m = excluded.position_y_m,
            position_z_m = excluded.position_z_m,
            velocity_x_m_s = excluded.velocity_x_m_s,
            velocity_y_m_s = excluded.velocity_y_m_s,
            velocity_z_m_s = excluded.velocity_z_m_s,
            orientation_q_w = excluded.orientation_q_w,
            orientation_q_x = excluded.orientation_q_x,
            orientation_q_y = excluded.orientation_q_y,
            orientation_q_z = excluded.orientation_q_z,
            angular_velocity_x_rad_s = excluded.angular_velocity_x_rad_s,
            angular_velocity_y_rad_s = excluded.angular_velocity_y_rad_s,
            angular_velocity_z_rad_s = excluded.angular_velocity_z_rad_s,
            reference_body_id = excluded.reference_body_id,
            captured_universe_epoch_s = excluded.captured_universe_epoch_s,
            captured_at_epoch_s = excluded.captured_at_epoch_s",
    )
    .bind(state.vehicle_id().to_string())
    .bind(state.position().raw().0)
    .bind(state.position().raw().1)
    .bind(state.position().raw().2)
    .bind(state.velocity().raw().0)
    .bind(state.velocity().raw().1)
    .bind(state.velocity().raw().2)
    .bind(state.orientation().w())
    .bind(state.orientation().x())
    .bind(state.orientation().y())
    .bind(state.orientation().z())
    .bind(state.angular_velocity().raw().0)
    .bind(state.angular_velocity().raw().1)
    .bind(state.angular_velocity().raw().2)
    .bind(state.reference_body_id().to_string())
    .bind(state.captured_universe_epoch().value())
    .bind(state.captured_at_epoch().value())
    .execute(pool)
    .await?;

    Ok(())
}