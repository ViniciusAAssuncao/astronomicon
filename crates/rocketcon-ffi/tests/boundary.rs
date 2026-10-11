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
    assert_eq!(engine.orbit_preview().unwrap_err(), "no vehicle loaded");
    assert_eq!(engine.vehicle_components().unwrap_err(), "no vehicle loaded");
    assert_eq!(engine.take_events().unwrap_err(), "no vehicle loaded");
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
    assert_eq!(initial.reference_body_id, PLANET_ID);
    let preview = engine.orbit_preview()?;
    assert_eq!(preview.reference_body_id, PLANET_ID);
    assert_eq!(preview.source_epoch_seconds, initial.total_epoch_seconds);
    assert!(preview.relative_points_m.len() >= 2);
    assert!(preview.relative_points_m.len() <= 193);
    assert!(preview.has_impact);
    if preview.has_inclination {
        assert!(preview.inclination_rad.is_finite());
        assert!((0.0..=std::f64::consts::PI).contains(&preview.inclination_rad));
    }
    if preview.has_ascending_longitude {
        assert!(preview.has_inclination);
        assert!(preview.ascending_longitude_rad.is_finite());
        assert!((0.0..std::f64::consts::TAU).contains(&preview.ascending_longitude_rad));
    }
    if preview.has_next_ascending {
        assert!(preview.has_ascending_longitude);
        assert!(preview.next_ascending_seconds.is_finite());
        assert!(preview.next_ascending_seconds >= 0.0);
    }
    if preview.has_next_descending {
        assert!(preview.has_ascending_longitude);
        assert!(preview.next_descending_seconds.is_finite());
        assert!(preview.next_descending_seconds >= 0.0);
    }
    assert!(preview.has_eccentricity);
    assert!(preview.eccentricity.is_finite());
    assert!(preview.eccentricity >= 1.0);
    if preview.has_orbit_view_basis {
        let horizontal = &preview.orbit_horizontal;
        let vertical = &preview.orbit_vertical;
        let h_norm = horizontal.x.hypot(horizontal.y).hypot(horizontal.z);
        let v_norm = vertical.x.hypot(vertical.y).hypot(vertical.z);
        let dot = horizontal.x * vertical.x +
            horizontal.y * vertical.y + horizontal.z * vertical.z;
        assert!((h_norm - 1.0).abs() < 1e-10);
        assert!((v_norm - 1.0).abs() < 1e-10);
        assert!(dot.abs() < 1e-10);
    }
    assert!(preview.has_soi);
    assert!(preview.soi_radius_m > 100_000_000.0);
    assert!(preview.soi_radius_m < 200_000_000.0);
    assert!(!preview.soi_outside);
    assert!(!preview.has_soi_exit);
    assert!(preview.has_equatorial_reference);
    let normal = &preview.equatorial_normal;
    let horizontal = &preview.equatorial_horizontal;
    let vertical = &preview.equatorial_vertical;
    let length = |v: &rocketcon_ffi::ffi::FfiVec3| v.x.hypot(v.y).hypot(v.z);
    let dot = |a: &rocketcon_ffi::ffi::FfiVec3, b: &rocketcon_ffi::ffi::FfiVec3|
        a.x * b.x + a.y * b.y + a.z * b.z;
    assert!((length(normal) - 1.0).abs() < 1e-10);
    assert!((length(horizontal) - 1.0).abs() < 1e-10);
    assert!((length(vertical) - 1.0).abs() < 1e-10);
    assert!(dot(normal, horizontal).abs() < 1e-10);
    assert!(dot(normal, vertical).abs() < 1e-10);
    assert!(preview.equatorial_has_inclination);
    assert!((0.0..=std::f64::consts::PI).contains(&preview.equatorial_inclination_rad));
    assert!(preview.has_periapsis);
    assert!(preview.periapsis_altitude_m.is_finite());
    assert!(!preview.has_apoapsis);
    assert!(!preview.has_period);
    assert!(initial.main_engine_loads.is_empty());
    assert!(initial.has_battery);
    assert_eq!(initial.battery_capacity_j, 0.0);
    assert!(initial.fuel_reserves.is_empty());
    assert!(initial.has_altitude);
    assert!(engine.take_events()?.is_empty());
    assert!((initial.altitude_m - 200_000.0).abs() < 1.0);
    assert!(initial.has_reference_speed);
    assert!(initial.has_reference_vertical_speed);
    assert!(initial.has_reference_horizontal_speed);
    assert!(initial.reference_horizontal_speed_m_s.is_finite());
    assert!(initial.has_local_up_body);
    assert!((initial.local_up_body.x.hypot(initial.local_up_body.y).hypot(initial.local_up_body.z) - 1.0).abs() < 1e-10);
    assert!(initial.reference_vertical_speed_m_s.is_finite());
    assert!(initial.reference_speed_m_s.is_finite());
    assert!(initial.has_reference_body);
    assert!(initial.reference_body_radius_m > 0.0);
    assert_eq!(initial.angular_velocity_rad_s.x, 0.01);
    assert_eq!(initial.angular_velocity_rad_s.y, -0.02);
    assert_eq!(initial.angular_velocity_rad_s.z, 0.03);
    assert!(engine.set_main_engine_load(VEHICLE_ID, 1.0).is_err());
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
    assert!(next.has_battery);
    assert!(next.has_reference_body);
    assert!(next.has_altitude);
    assert!(next.has_reference_speed);
    assert!(next.has_reference_vertical_speed);
    assert!(next.has_reference_horizontal_speed);
    assert!(next.reference_horizontal_speed_m_s.is_finite());
    assert!(next.has_local_up_body);
    assert!(next.reference_vertical_speed_m_s.is_finite());
    assert!(engine.take_events()?.is_empty());
    assert!(next.reference_speed_m_s.is_finite());
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

#[test]
fn ffi_exposes_inclination_and_nodal_crossings() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-ffi-nodes-{}.db", Uuid::new_v4()));
    prepare_save(&path)?;
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let pool = astronomicon_db::connection::open_pool_path(&path).await?;
        let epoch = rocketcon_app::universe::resolve_universe_epoch(&pool).await?;
        let meros = Uuid::parse_str(PLANET_ID)?;
        let environment = rocketcon_app::environment::load_environment_snapshot(
            &pool, meros, epoch, Duration::new(1.0)).await?;
        let radius = environment.planet.equatorial_radius().ok_or("missing radius")?.value() + 200_000.0;
        let speed = (6.67430e-11 * environment.planet.mass().value() / radius).sqrt();
        let (_, body_velocity) = rocketcon_app::orbital::soi::resolve_body_state_at_epoch(
            &pool, meros, environment.system_id, epoch + Duration::new(1.0)).await?;
        let body_velocity = body_velocity.raw();
        let inclination = 45.0_f64.to_radians();
        sqlx::query("UPDATE vehicle_physical_states SET velocity_x_m_s = ?, velocity_y_m_s = ?, velocity_z_m_s = ? WHERE vehicle_id = ?")
            .bind(body_velocity.0)
            .bind(body_velocity.1 + speed * inclination.cos())
            .bind(body_velocity.2 + speed * inclination.sin())
            .bind(VEHICLE_ID)
            .execute(&pool).await?;
        pool.close().await;
        Ok::<_, Box<dyn Error>>(())
    })?;
    let mut engine = create_engine()?;
    engine.load_save(path.to_str().ok_or("invalid path")?, VEHICLE_ID)?;
    let preview = engine.orbit_preview()?;
    assert!(preview.has_inclination);
    assert!((preview.inclination_rad.to_degrees() - 45.0).abs() < 1e-5);
    assert!(preview.has_ascending_longitude);
    assert!(preview.ascending_longitude_rad.abs() < 1e-5);
    assert!(preview.has_next_descending);
    assert!(preview.next_descending_seconds > 0.0);
    assert!(preview.next_descending_position_m.z.abs() < 1e-5);
    drop(engine);
    let _ = std::fs::remove_file(path);
    Ok(())
}

#[test]
fn ffi_does_not_invent_equator_when_planet_data_is_missing() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-ffi-no-equator-{}.db", Uuid::new_v4()));
    prepare_save(&path)?;
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let pool = astronomicon_db::connection::open_pool_path(&path).await?;
        let changed = sqlx::query("UPDATE planets SET axial_tilt_rad = NULL WHERE name = 'Meros'")
            .execute(&pool).await?;
        assert_eq!(changed.rows_affected(), 1);
        pool.close().await;
        Ok::<_, Box<dyn Error>>(())
    })?;
    let mut engine = create_engine()?;
    engine.load_save(path.to_str().ok_or("invalid path")?, VEHICLE_ID)?;
    let preview = engine.orbit_preview()?;
    assert!(!preview.has_equatorial_reference);
    assert!(!preview.equatorial_has_inclination);
    assert!(!preview.equatorial_has_next_ascending);
    assert!(preview.has_inclination);
    drop(engine);
    let _ = std::fs::remove_file(path);
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
