use astronomicon_core::units::Duration;
use rocketcon_ffi::create_engine;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

const VEHICLE_ID: &str = "e2897c6a-7d04-4ebc-882c-87984a74a200";
const PLANET_ID: &str = "60dc1b97-29a6-4b1c-95fd-42b229079928";
const COMPONENT_ID: &str = "56c1b894-4718-4f0c-94a0-bc22a6053180";

#[test]
fn ffi_loads_steps_and_exposes_snapshot() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-ffi-{}.db", Uuid::new_v4()));
    prepare_save(&path)?;
    let mut engine = create_engine()?;
    assert_eq!(engine.snapshot().unwrap_err(), "no vehicle loaded");
    assert_eq!(engine.vehicle_components().unwrap_err(), "no vehicle loaded");
    engine.load_save(path.to_str().ok_or("invalid path")?, VEHICLE_ID)?;
    let components = engine.vehicle_components()?;
    assert_eq!(components.len(), 2);
    let mounted = components.iter().find(|part| part.stage_index == 1).ok_or("stage missing")?;
    assert_eq!(mounted.name, "Upper CPU");
    assert_eq!(mounted.kind, 4);
    assert_eq!(mounted.offset_x_m, 2.0);
    assert_eq!(mounted.offset_z_m, 3.0);
    assert_eq!(mounted.length_m, 1.0);
    assert_eq!(mounted.diameter_m, 1.0);
    let initial = engine.snapshot()?;
    assert!(!initial.has_altitude);
    assert!(initial.has_reference_body);
    assert!(initial.reference_body_radius_m > 0.0);
    assert_eq!(initial.angular_velocity_rad_s.x, 0.01);
    assert_eq!(initial.angular_velocity_rad_s.y, -0.02);
    assert_eq!(initial.angular_velocity_rad_s.z, 0.03);
    engine.set_control(0.0, 0.0, 0.0)?;
    assert_eq!(
        engine.step(0.0).unwrap_err(),
        "tick duration must be positive and finite"
    );
    engine.step(0.02)?;
    let next = engine.snapshot()?;
    assert!(next.total_epoch_seconds > initial.total_epoch_seconds);
    assert!(next.speed_m_s.is_finite());
    assert!(next.angular_velocity_rad_s.x.is_finite());
    assert!(next.angular_velocity_rad_s.y.is_finite());
    assert!(next.angular_velocity_rad_s.z.is_finite());
    assert!(next.has_g_load);
    assert!(next.has_reference_body);
    assert!(engine.load_save("missing-save.db", VEHICLE_ID).is_err());
    assert_eq!(engine.snapshot()?.position_m.x, next.position_m.x);
    engine.save()?;
    drop(engine);
    let mut reopened = create_engine()?;
    reopened.load_save(path.to_str().ok_or("invalid path")?, VEHICLE_ID)?;
    let persisted = reopened.snapshot()?;
    assert_eq!(persisted.position_m, next.position_m);
    assert_eq!(persisted.total_epoch_seconds, next.total_epoch_seconds);
    Ok(())
}

fn prepare_save(path: &Path) -> Result<(), Box<dyn Error>> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/astronomicon.db");
        let options = SqliteConnectOptions::new().filename(template).read_only(true);
        let mut source = SqliteConnection::connect_with(&options).await?;
        sqlx::query("VACUUM INTO ?")
            .bind(path.to_str().ok_or("invalid path")?)
            .execute(&mut source).await?;
        source.close().await?;
        let pool = astronomicon_db::connection::open_pool_path(path).await?;
        rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
        let epoch = rocketcon_app::universe::resolve_universe_epoch(&pool).await?;
        let environment = rocketcon_app::environment::load_environment_snapshot(
            &pool, Uuid::parse_str(PLANET_ID)?, epoch, Duration::new(1.0),
        ).await?;
        let radius = environment.planet.equatorial_radius().ok_or("missing radius")?.value();
        let center = environment.planet_position.raw();
        let position = center.0 + radius + 200_000.0;
        sqlx::query("INSERT INTO vehicles (id, name, vehicle_kind) VALUES (?, 'FFI Probe', 'Probe')")
            .bind(VEHICLE_ID).execute(&pool).await?;
        sqlx::query("INSERT INTO components (id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w) VALUES (?, 'FFI CPU', 'Cpu', 100.0, 1.0, 1.0, 0.0)")
            .bind(COMPONENT_ID).execute(&pool).await?;
        sqlx::query("INSERT INTO vehicle_components (id, vehicle_id, component_id, stage_index) VALUES (?, ?, ?, 0)")
            .bind(Uuid::new_v4().to_string()).bind(VEHICLE_ID).bind(COMPONENT_ID)
            .execute(&pool).await?;
        sqlx::query("INSERT INTO vehicle_components (id, vehicle_id, component_id, stage_index, instance_label, mount_offset_x_m, mount_offset_z_m) VALUES (?, ?, ?, 1, 'Upper CPU', 2.0, 3.0)")
            .bind(Uuid::new_v4().to_string()).bind(VEHICLE_ID).bind(COMPONENT_ID)
            .execute(&pool).await?;
        sqlx::query("INSERT INTO vehicle_physical_states (vehicle_id, position_x_m, position_y_m, position_z_m, velocity_x_m_s, velocity_y_m_s, velocity_z_m_s, orientation_q_w, orientation_q_x, orientation_q_y, orientation_q_z, angular_velocity_x_rad_s, angular_velocity_y_rad_s, angular_velocity_z_rad_s, reference_body_id, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, ?, ?, ?, 0, 0, 0, 1, 0, 0, 0, 0.01, -0.02, 0.03, ?, ?, 1)")
            .bind(VEHICLE_ID).bind(position).bind(center.1).bind(center.2)
            .bind(PLANET_ID).bind(epoch.value()).execute(&pool).await?;
        pool.close().await;
        Ok::<_, Box<dyn Error>>(())
    })
}
