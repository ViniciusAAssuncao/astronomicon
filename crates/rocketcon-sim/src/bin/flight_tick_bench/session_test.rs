use astronomicon_db::connection::open_pool_path;
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_sim::RocketconSession;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

#[tokio::test]
async fn session_drives_vehicle_and_reloads_saved_state() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-session-{}.db", Uuid::new_v4()));
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
    let pool = open_pool_path(&path).await?;
    rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
    let (vehicle_id, _) = super::create_fixture(&pool, super::Scenario::Atmosphere).await?;
    pool.close().await;

    let mut session = RocketconSession::load(&path, vehicle_id).await?;
    let initial = session.snapshot().clone();
    assert_eq!(session.trajectory(), &[initial.clone()]);
    assert!(session.step(0.0).await.is_err());
    assert_eq!(session.snapshot(), &initial);
    session.apply_control(VehicleControlInput::new().with_pitch_yaw_roll(0.0, 0.0, 0.0));
    let first = session.step(super::DT).await?.clone();
    assert!(first.total_epoch_seconds > initial.total_epoch_seconds);
    assert!(first.speed_m_s.is_finite());
    assert!(first.mach.is_some());
    assert_eq!(session.trajectory().len(), 2);
    session.save().await?;
    session.close().await;

    let reopened = RocketconSession::load(&path, vehicle_id).await?;
    assert_eq!(reopened.snapshot().position_m, first.position_m);
    assert_eq!(
        reopened.snapshot().total_epoch_seconds,
        first.total_epoch_seconds
    );
    reopened.close().await;
    Ok(())
}
