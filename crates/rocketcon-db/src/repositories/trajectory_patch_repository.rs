use crate::error::RocketDbResult;
use crate::models::trajectory_patch_row::{format_float_list, TrajectoryPatchRow};
use astronomicon_core::units::Duration;
use rocketcon_core::domain::{TrajectoryPatch, TrajectoryPatchKind};
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, vehicle_id, reference_body_id, start_universe_epoch_s, end_universe_epoch_s, gravitational_parameter_m3_s2, patch_type, semi_major_axis_m, eccentricity, inclination_rad, longitude_of_ascending_node_rad, argument_of_periapsis_rad, true_anomaly_at_epoch_rad, initial_mass_kg, final_mass_kg, thrust_n, specific_impulse_s, total_delta_v_m_s, chebyshev_x_json, chebyshev_y_json, chebyshev_z_json, chebyshev_vx_json, chebyshev_vy_json, chebyshev_vz_json, chebyshev_mass_json FROM vehicle_trajectory_patches";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> RocketDbResult<Option<TrajectoryPatch>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let row = sqlx::query_as::<_, TrajectoryPatchRow>(&query)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(TrajectoryPatch::try_from).transpose()
}

pub async fn list_for_vehicle(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
) -> RocketDbResult<Vec<TrajectoryPatch>> {
    let query = format!("{BASE_QUERY} WHERE vehicle_id = ? ORDER BY start_universe_epoch_s ASC");
    let rows = sqlx::query_as::<_, TrajectoryPatchRow>(&query)
        .bind(vehicle_id.to_string())
        .fetch_all(pool)
        .await?;

    rows.into_iter().map(TrajectoryPatch::try_from).collect()
}

pub async fn find_patch_at_epoch(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
    epoch: Duration,
) -> RocketDbResult<Option<TrajectoryPatch>> {
    let query = format!(
        "{BASE_QUERY} WHERE vehicle_id = ? AND start_universe_epoch_s <= ? AND (end_universe_epoch_s IS NULL OR end_universe_epoch_s >= ?) ORDER BY start_universe_epoch_s DESC LIMIT 1"
    );
    let row = sqlx::query_as::<_, TrajectoryPatchRow>(&query)
        .bind(vehicle_id.to_string())
        .bind(epoch.value())
        .bind(epoch.value())
        .fetch_optional(pool)
        .await?;

    row.map(TrajectoryPatch::try_from).transpose()
}

pub async fn insert_patch(pool: &SqlitePool, patch: &TrajectoryPatch) -> RocketDbResult<()> {
    match patch.kind() {
        TrajectoryPatchKind::Conic(conic) => {
            sqlx::query(
                "INSERT INTO vehicle_trajectory_patches (
                    id, vehicle_id, reference_body_id,
                    start_universe_epoch_s, end_universe_epoch_s,
                    gravitational_parameter_m3_s2, patch_type,
                    semi_major_axis_m, eccentricity, inclination_rad,
                    longitude_of_ascending_node_rad, argument_of_periapsis_rad,
                    true_anomaly_at_epoch_rad,
                    initial_mass_kg, final_mass_kg, thrust_n,
                    specific_impulse_s, total_delta_v_m_s,
                    chebyshev_x_json, chebyshev_y_json, chebyshev_z_json,
                    chebyshev_vx_json, chebyshev_vy_json, chebyshev_vz_json,
                    chebyshev_mass_json
                ) VALUES (?, ?, ?, ?, ?, ?, 'conic', ?, ?, ?, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL)",
            )
            .bind(patch.id().to_string())
            .bind(patch.vehicle_id().to_string())
            .bind(patch.reference_body_id().to_string())
            .bind(patch.start_universe_epoch().value())
            .bind(patch.end_universe_epoch().map(|d| d.value()))
            .bind(patch.gravitational_parameter().value())
            .bind(conic.semi_major_axis.value())
            .bind(conic.eccentricity)
            .bind(conic.inclination.value())
            .bind(conic.longitude_of_ascending_node.value())
            .bind(conic.argument_of_periapsis.value())
            .bind(conic.true_anomaly_at_epoch.value())
            .execute(pool)
            .await?;
        }
        TrajectoryPatchKind::LowThrust(lt) => {
            sqlx::query(
                "INSERT INTO vehicle_trajectory_patches (
                    id, vehicle_id, reference_body_id,
                    start_universe_epoch_s, end_universe_epoch_s,
                    gravitational_parameter_m3_s2, patch_type,
                    semi_major_axis_m, eccentricity, inclination_rad,
                    longitude_of_ascending_node_rad, argument_of_periapsis_rad,
                    true_anomaly_at_epoch_rad,
                    initial_mass_kg, final_mass_kg, thrust_n,
                    specific_impulse_s, total_delta_v_m_s,
                    chebyshev_x_json, chebyshev_y_json, chebyshev_z_json,
                    chebyshev_vx_json, chebyshev_vy_json, chebyshev_vz_json,
                    chebyshev_mass_json
                ) VALUES (?, ?, ?, ?, ?, ?, 'low_thrust', NULL, NULL, NULL, NULL, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(patch.id().to_string())
            .bind(patch.vehicle_id().to_string())
            .bind(patch.reference_body_id().to_string())
            .bind(patch.start_universe_epoch().value())
            .bind(patch.end_universe_epoch().map(|d| d.value()))
            .bind(patch.gravitational_parameter().value())
            .bind(lt.initial_mass.value())
            .bind(lt.final_mass.value())
            .bind(lt.thrust.value())
            .bind(lt.specific_impulse.value())
            .bind(lt.total_delta_v.value())
            .bind(format_float_list(&lt.chebyshev_x))
            .bind(format_float_list(&lt.chebyshev_y))
            .bind(format_float_list(&lt.chebyshev_z))
            .bind(format_float_list(&lt.chebyshev_vx))
            .bind(format_float_list(&lt.chebyshev_vy))
            .bind(format_float_list(&lt.chebyshev_vz))
            .bind(format_float_list(&lt.chebyshev_mass))
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

pub async fn insert_patches(pool: &SqlitePool, patches: &[TrajectoryPatch]) -> RocketDbResult<()> {
    for patch in patches {
        insert_patch(pool, patch).await?;
    }
    Ok(())
}

pub async fn delete_patches_for_vehicle(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
) -> RocketDbResult<()> {
    sqlx::query("DELETE FROM vehicle_trajectory_patches WHERE vehicle_id = ?")
        .bind(vehicle_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_future_patches_after_epoch(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
    epoch: Duration,
) -> RocketDbResult<()> {
    sqlx::query(
        "DELETE FROM vehicle_trajectory_patches WHERE vehicle_id = ? AND start_universe_epoch_s >= ?",
    )
    .bind(vehicle_id.to_string())
    .bind(epoch.value())
    .execute(pool)
    .await?;
    Ok(())
}