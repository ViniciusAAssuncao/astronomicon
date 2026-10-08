use crate::error::RocketDbResult;
use crate::models::ThermalNodeStateRow;
use rocketcon_core::domain::ThermalNodeState;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT vehicle_component_id, current_temperature_k, captured_universe_epoch_s, captured_at_epoch_s FROM thermal_node_states";
const UPSERT_QUERY: &str = "INSERT INTO thermal_node_states (
            vehicle_component_id,
            current_temperature_k,
            captured_universe_epoch_s,
            captured_at_epoch_s
        ) VALUES (?, ?, ?, ?)
        ON CONFLICT(vehicle_component_id) DO UPDATE SET
            current_temperature_k = excluded.current_temperature_k,
            captured_universe_epoch_s = excluded.captured_universe_epoch_s,
            captured_at_epoch_s = excluded.captured_at_epoch_s";

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

pub async fn upsert(pool: &SqlitePool, state: &ThermalNodeState) -> RocketDbResult<()> {
    sqlx::query(UPSERT_QUERY)
        .bind(state.vehicle_component_id().to_string())
        .bind(state.current_temperature_k())
        .bind(state.captured_universe_epoch().value())
        .bind(state.captured_at_epoch().value())
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn upsert_many_atomic(
    pool: &SqlitePool,
    states: &[ThermalNodeState],
) -> RocketDbResult<()> {
    if states.is_empty() {
        return Ok(());
    }
    let mut transaction = pool.begin().await?;
    for state in states {
        upsert_on_connection(&mut transaction, state).await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn upsert_many_in_existing_transaction(
    pool: &SqlitePool,
    states: &[ThermalNodeState],
) -> RocketDbResult<()> {
    for state in states {
        upsert(pool, state).await?;
    }
    Ok(())
}

async fn upsert_on_connection(
    connection: &mut SqliteConnection,
    state: &ThermalNodeState,
) -> RocketDbResult<()> {
    sqlx::query(UPSERT_QUERY)
        .bind(state.vehicle_component_id().to_string())
        .bind(state.current_temperature_k())
        .bind(state.captured_universe_epoch().value())
        .bind(state.captured_at_epoch().value())
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, vehicle_component_id: &Uuid) -> RocketDbResult<()> {
    sqlx::query("DELETE FROM thermal_node_states WHERE vehicle_component_id = ?")
        .bind(vehicle_component_id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::{Duration, Temperature};
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn batch_rolls_back_all_nodes_when_one_write_fails()
    -> Result<(), Box<dyn std::error::Error>> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        sqlx::query(
            "CREATE TABLE thermal_node_states (
                vehicle_component_id TEXT PRIMARY KEY,
                current_temperature_k REAL NOT NULL CHECK (current_temperature_k <= 500),
                captured_universe_epoch_s REAL NOT NULL,
                captured_at_epoch_s REAL NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        let first_id = Uuid::new_v4();
        let second_id = Uuid::new_v4();
        let state = |id, kelvin| {
            ThermalNodeState::new(
                id,
                Temperature::new(kelvin),
                Duration::new(0.0),
                Duration::new(1.0),
            )
        };
        upsert(&pool, &state(first_id, 300.0)?).await?;
        upsert(&pool, &state(second_id, 300.0)?).await?;

        let failed =
            upsert_many_atomic(&pool, &[state(first_id, 310.0)?, state(second_id, 600.0)?]).await;
        assert!(failed.is_err());
        assert_eq!(
            get_by_vehicle_component_id(&pool, &first_id)
                .await?
                .ok_or("first node missing")?
                .current_temperature_k(),
            300.0
        );
        assert_eq!(
            get_by_vehicle_component_id(&pool, &second_id)
                .await?
                .ok_or("second node missing")?
                .current_temperature_k(),
            300.0
        );

        upsert_many_atomic(&pool, &[state(first_id, 310.0)?, state(second_id, 320.0)?]).await?;
        assert_eq!(
            get_by_vehicle_component_id(&pool, &first_id)
                .await?
                .ok_or("first node missing after update")?
                .current_temperature_k(),
            310.0
        );
        assert_eq!(
            get_by_vehicle_component_id(&pool, &second_id)
                .await?
                .ok_or("second node missing after update")?
                .current_temperature_k(),
            320.0
        );
        Ok(())
    }
}
