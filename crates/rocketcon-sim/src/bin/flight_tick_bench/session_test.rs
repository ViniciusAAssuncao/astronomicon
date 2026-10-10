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

#[tokio::test]
async fn throttle_ramp_advances_with_ticks_and_stops_on_release() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-ramp-{}.db", Uuid::new_v4()));
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/astronomicon.db");
    let options = SqliteConnectOptions::new().filename(template).read_only(true);
    let mut source = SqliteConnection::connect_with(&options).await?;
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_str().ok_or("invalid temporary database path")?)
        .execute(&mut source).await?;
    source.close().await?;
    let pool = open_pool_path(&path).await?;
    rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
    let (vehicle_id, _) = super::create_fixture(&pool, super::Scenario::Powered).await?;
    sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, numeric_value) VALUES (?, 'min_throttle_fraction', 0.01)")
        .bind(super::ENGINE_ID).execute(&pool).await?;
    pool.close().await;

    let engine = Uuid::from_u128(0xe2897c6a7d044ebc882c87984a740102);
    let mut session = RocketconSession::load(&path, vehicle_id).await?;
    let load = |session: &RocketconSession| session.snapshot().main_engine_loads[0].1;
    assert!(session.set_main_engine_ramp(engine, 2).await.is_err());
    session.set_main_engine_ramp(engine, 1).await?;
    assert!((load(&session) - 0.51).abs() < 1e-9);
    session.step(0.02).await?;
    assert!((load(&session) - 0.53).abs() < 1e-9, "{}", load(&session));
    session.set_main_engine_ramp(engine, 0).await?;
    session.step(0.02).await?;
    assert!((load(&session) - 0.53).abs() < 1e-9);
    session.set_main_engine_ramp(engine, -1).await?;
    assert!((load(&session) - 0.52).abs() < 1e-9);
    session.step(0.02).await?;
    assert!((load(&session) - 0.50).abs() < 1e-9);
    session.set_main_engine_load(engine, 0.0).await?;
    assert!(session.set_main_engine_ramp(engine, 1).await.is_err());
    session.step(0.02).await?;
    assert_eq!(load(&session), 0.0);
    session.set_main_engine_ramp(engine, 0).await?;
    session.close().await;

    let reopened = RocketconSession::load(&path, vehicle_id).await?;
    assert_eq!(load(&reopened), 0.0);
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn orbital_preview_is_sampled_without_advancing_the_save() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-orbit-preview-{}.db", Uuid::new_v4()));
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/astronomicon.db");
    let options = SqliteConnectOptions::new().filename(template).read_only(true);
    let mut source = SqliteConnection::connect_with(&options).await?;
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_str().ok_or("invalid temporary database path")?)
        .execute(&mut source).await?;
    source.close().await?;
    let pool = open_pool_path(&path).await?;
    rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
    let (vehicle_id, _) = super::create_fixture(&pool, super::Scenario::Coast).await?;
    pool.close().await;

    let session = RocketconSession::load(&path, vehicle_id).await?;
    let before = session.snapshot().clone();
    let preview = session.orbit_preview().await?;
    assert_eq!(preview.reference_body_id, before.reference_body_id);
    assert_eq!(preview.source_epoch_seconds, before.total_epoch_seconds);
    assert_eq!(preview.relative_points_m.len(), 193);
    assert!(preview.horizon_seconds >= 600.0 && preview.horizon_seconds <= 21_600.0);
    assert!(preview.relative_points_m.iter().flatten().all(|value| value.is_finite()));
    for pair in preview.relative_points_m.windows(2) {
        let jump = (0..3).map(|axis| (pair[1][axis] - pair[0][axis]).powi(2)).sum::<f64>().sqrt();
        assert!(jump < 500_000.0, "discontinuous orbital preview: {jump} m");
    }
    let body = before.reference_body_position_m.ok_or("missing body position")?;
    for axis in 0..3 {
        assert!((preview.relative_points_m[0][axis] - (before.position_m[axis] - body[axis])).abs() < 1.0);
    }
    assert_eq!(session.snapshot(), &before);
    session.close().await;
    let reopened = RocketconSession::load(&path, vehicle_id).await?;
    assert_eq!(reopened.snapshot().total_epoch_seconds, before.total_epoch_seconds);
    reopened.close().await;
    Ok(())
}
