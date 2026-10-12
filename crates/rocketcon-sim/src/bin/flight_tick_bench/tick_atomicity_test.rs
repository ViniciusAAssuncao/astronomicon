use astronomicon_core::units::Duration;
use astronomicon_db::connection::open_pool;
use rocketcon_core::domain::VehicleControlInput;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, Row, SqliteConnection, SqlitePool};
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

#[tokio::test]
async fn late_tick_failure_rolls_back_every_table() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-tick-atomic-{}.db", Uuid::new_v4()));
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/astronomicon.db");
    let options = SqliteConnectOptions::new()
        .filename(template)
        .read_only(true);
    let mut source = SqliteConnection::connect_with(&options).await?;
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_str().ok_or("invalid temporary database path")?)
        .execute(&mut source)
        .await?;
    source.close().await?;

    let pool = open_pool(&format!("sqlite://{}", path.display())).await?;
    rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
    let (vehicle_id, epoch) = super::create_fixture(&pool, super::Scenario::Powered).await?;
    sqlx::query(
        "CREATE TRIGGER abort_thermal_tick BEFORE INSERT ON thermal_node_states \
         BEGIN SELECT RAISE(ABORT, 'forced late failure'); END",
    )
    .execute(&pool)
    .await?;

    let before = database_snapshot(&pool).await?;
    let input = VehicleControlInput::new();
    let failed = rocketcon_app::aeroespacial::advance_vehicle_simulation(
        &pool,
        vehicle_id,
        Duration::new(super::DT),
        epoch,
        &input,
    )
    .await;
    assert!(failed.is_err());
    assert_eq!(before, database_snapshot(&pool).await?);

    sqlx::query("DROP TRIGGER abort_thermal_tick")
        .execute(&pool)
        .await?;
    let report = rocketcon_app::aeroespacial::advance_vehicle_simulation(
        &pool,
        vehicle_id,
        Duration::new(super::DT),
        epoch,
        &input,
    )
    .await?;
    super::validate(&report, super::INITIAL_AT_EPOCH + super::DT)?;

    sqlx::query(
        "CREATE TRIGGER abort_thermal_tick_again BEFORE INSERT ON thermal_node_states \
         BEGIN SELECT RAISE(ABORT, 'forced session failure'); END",
    )
    .execute(&pool)
    .await?;
    let before_session_failure = database_snapshot(&pool).await?;
    let session = rocketcon_db::tick_transaction::TickTransactionSession::new(&pool).await?;
    let session_failure = rocketcon_app::aeroespacial::advance_vehicle_simulation_in_session(
        &session,
        vehicle_id,
        Duration::new(super::DT),
        epoch,
        &input,
    )
    .await;
    assert!(session_failure.is_err());
    assert_eq!(before_session_failure, database_snapshot(&pool).await?);
    session.close().await;
    sqlx::query("DROP TRIGGER abort_thermal_tick_again")
        .execute(&pool)
        .await?;

    let (first, second) = tokio::join!(
        rocketcon_app::aeroespacial::advance_vehicle_simulation(
            &pool,
            vehicle_id,
            Duration::new(super::DT),
            epoch,
            &input,
        ),
        rocketcon_app::aeroespacial::advance_vehicle_simulation(
            &pool,
            vehicle_id,
            Duration::new(super::DT),
            epoch,
            &input,
        )
    );
    first?;
    second?;
    let persisted =
        rocketcon_db::repositories::vehicle_physical_state::get_by_vehicle_id(&pool, &vehicle_id)
            .await?
            .ok_or("physical state missing after concurrent ticks")?;
    assert!(
        (persisted.captured_at_epoch().value() - (super::INITIAL_AT_EPOCH + 3.0 * super::DT)).abs()
            < 1e-8
    );

    pool.close().await;
    drop(pool);
    for attempt in 0..10 {
        match std::fs::remove_file(&path) {
            Ok(()) => break,
            Err(error) if attempt < 9 => {
                let _ = error;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

async fn database_snapshot(pool: &SqlitePool) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let tables = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    let mut snapshot = Vec::with_capacity(tables.len());
    for table in tables {
        let name: String = table.try_get("name")?;
        let identifier = format!("\"{}\"", name.replace('"', "\"\""));
        let columns = sqlx::query(&format!("PRAGMA table_info({identifier})"))
            .fetch_all(pool)
            .await?;
        let expressions = columns
            .iter()
            .map(|column| {
                let column_name: String = column.try_get("name")?;
                Ok(format!("quote(\"{}\")", column_name.replace('"', "\"\"")))
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()?
            .join(", ");
        let query = format!(
            "SELECT COALESCE(json_group_array(json_array({expressions})), '[]') \
             FROM (SELECT * FROM {identifier} ORDER BY rowid)"
        );
        let contents: String = sqlx::query_scalar(&query).fetch_one(pool).await?;
        snapshot.push((name, contents));
    }
    Ok(snapshot)
}
