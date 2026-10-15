use astronomicon_core::units::Duration;
use astronomicon_db::connection::open_pool_path;
use rocketcon_core::domain::{VehicleControlInput, VehicleSnapshot};
use rocketcon_sim::RocketconSession;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use std::collections::HashMap;
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
    let (vehicle_id, epoch) = super::create_fixture(&pool, super::Scenario::Atmosphere).await?;
    let reference_id = Uuid::parse_str(super::HADAB_ID)?;
    for at_epoch in [
        Duration::new(super::INITIAL_AT_EPOCH),
        Duration::new(super::INITIAL_AT_EPOCH + super::DT),
    ] {
        let environment = rocketcon_app::environment::load_environment_snapshot(
            &pool,
            reference_id,
            epoch,
            at_epoch,
        )
        .await?;
        let direct_positions = astronomicon_app::ephemeris::resolve_system_positions(
            &pool,
            environment.system_id,
            epoch + at_epoch,
        )
        .await?;
        assert_eq!(
            environment.reference_position(),
            *direct_positions
                .get(&reference_id)
                .ok_or("reference position missing")?
        );
        assert_eq!(
            environment.star_position,
            *direct_positions
                .get(&environment.star.id())
                .ok_or("parent star position missing")?
        );
        let star_environment = rocketcon_app::environment::load_environment_snapshot(
            &pool,
            environment.star.id(),
            epoch,
            at_epoch,
        )
        .await?;
        assert_eq!(
            star_environment.reference_position(),
            environment.star_position
        );
        assert!(star_environment.planet.is_none());

        let catalog =
            astronomicon_app::ephemeris::EphemerisCatalog::load(&pool, environment.system_id)
                .await?;
        let source_ids = catalog
            .gravitating_bodies()
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        let prepared = catalog.prepared_positions()?;
        let mut workspace = prepared.workspace(source_ids.len());
        let (primary, secondary): (String, String) = sqlx::query_as(
            "SELECT primary_star_id, secondary_star_id FROM barycenters WHERE star_system_id = ? LIMIT 1",
        )
        .bind(environment.system_id.to_string())
        .fetch_one(&pool)
        .await?;
        let barycenter_members = [Uuid::parse_str(&primary)?, Uuid::parse_str(&secondary)?];
        for offset in [0.0, 0.01, 1.0, 128.0] {
            let full = catalog.positions_at_offset(epoch + at_epoch, offset)?;
            let members =
                prepared.selected_at_offset(epoch + at_epoch, offset, &barycenter_members)?;
            for id in barycenter_members {
                assert_eq!(members.get(&id), full.get(&id));
            }
            let direct =
                catalog.positions_for_ids_at_offset(epoch + at_epoch, offset, &source_ids)?;
            let reused = prepared.selected_at_offset(epoch + at_epoch, offset, &source_ids)?;
            prepared.selected_at_offset_into(
                epoch + at_epoch,
                offset,
                &source_ids,
                &mut workspace,
            )?;
            assert_eq!(direct, reused);
            assert_eq!(workspace.positions().len(), source_ids.len());
            for (id, position) in source_ids.iter().zip(workspace.positions()) {
                assert_eq!(direct.get(id), Some(position));
            }
        }
        let repeated = [source_ids[0], source_ids[0]];
        assert!(
            prepared
                .selected_at_offset_into(epoch + at_epoch, 0.0, &repeated, &mut workspace)
                .is_err()
        );
        assert!(workspace.positions().is_empty());
        assert!(
            prepared
                .selected_at_offset_into(epoch + at_epoch, f64::NAN, &source_ids, &mut workspace)
                .is_err()
        );
        assert!(workspace.positions().is_empty());
        let mut missing_source = source_ids.clone();
        missing_source.push(Uuid::nil());
        assert!(
            prepared
                .selected_at_offset_into(epoch + at_epoch, 0.0, &missing_source, &mut workspace)
                .is_err()
        );
        assert!(workspace.positions().is_empty());
        prepared.selected_at_offset_into(epoch + at_epoch, 0.0, &source_ids, &mut workspace)?;
        assert_eq!(workspace.positions().len(), source_ids.len());
        assert!(
            prepared
                .selected_at_offset(epoch + at_epoch, 0.0, &[])
                .is_err()
        );
    }
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
    let (vehicle_id, _) = super::create_fixture(&pool, super::Scenario::Powered).await?;
    sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, numeric_value) VALUES (?, 'min_throttle_fraction', 0.01)")
        .bind(super::ENGINE_ID).execute(&pool).await?;
    pool.close().await;

    let engine = Uuid::from_u128(0xe2897c6a7d044ebc882c87984a740102);
    let mut session = RocketconSession::load(&path, vehicle_id).await?;
    let load = |session: &RocketconSession| session.snapshot().main_engine_loads[0].1;
    let powered_preview = session.orbit_preview().await?;
    assert!(powered_preview.under_thrust);
    assert!(powered_preview.relative_points_m.len() > 2);
    assert!(powered_preview.periapsis_altitude_m.is_some());
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
    let coast_preview = session.orbit_preview().await?;
    assert!(!coast_preview.under_thrust);
    assert!(!coast_preview.relative_points_m.is_empty());
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
    let (vehicle_id, _) = super::create_fixture(&pool, super::Scenario::Coast).await?;
    pool.close().await;

    let session = RocketconSession::load(&path, vehicle_id).await?;
    let before = session.snapshot().clone();
    let preview = session.orbit_preview().await?;
    assert_eq!(preview.reference_body_id, before.reference_body_id);
    assert_eq!(preview.source_epoch_seconds, before.total_epoch_seconds);
    assert!((3..=257).contains(&preview.relative_points_m.len()));
    assert!(preview.rendered_horizon_seconds > 0.0);
    assert!((preview.rendered_horizon_seconds - preview.horizon_seconds).abs() < 1e-3);
    assert!(!preview.under_thrust);
    assert!(preview.eccentricity.ok_or("eccentricity missing")? < 1e-7);
    assert!((preview.periapsis_altitude_m.ok_or("periapsis missing")? - 200_000.0).abs() < 1.0);
    assert!((preview.apoapsis_altitude_m.ok_or("apoapsis missing")? - 200_000.0).abs() < 1.0);
    assert!(preview.period_seconds.ok_or("period missing")? > 0.0);
    assert!(preview.horizon_seconds >= 600.0 && preview.horizon_seconds <= 21_600.0);
    assert!(
        preview
            .relative_points_m
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
    for pair in preview.relative_points_m.windows(2) {
        let jump = (0..3)
            .map(|axis| (pair[1][axis] - pair[0][axis]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(jump < 500_000.0, "discontinuous orbital preview: {jump} m");
    }
    let body = before
        .reference_body_position_m
        .ok_or("missing body position")?;
    for axis in 0..3 {
        assert!(
            (preview.relative_points_m[0][axis] - (before.position_m[axis] - body[axis])).abs()
                < 1.0
        );
    }
    assert_eq!(session.snapshot(), &before);
    session.close().await;
    let reopened = RocketconSession::load(&path, vehicle_id).await?;
    assert_eq!(
        reopened.snapshot().total_epoch_seconds,
        before.total_epoch_seconds
    );
    reopened.close().await;
    Ok(())
}

#[tokio::test]
async fn selective_snapshot_matches_full_state_reference() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-snapshot-{}.db", Uuid::new_v4()));
    let template = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../database/astronomicon.db");
    let mut source = SqliteConnection::connect_with(
        &SqliteConnectOptions::new()
            .filename(template)
            .read_only(true),
    )
    .await?;
    sqlx::query("VACUUM INTO ?")
        .bind(path.to_str().ok_or("invalid temporary database path")?)
        .execute(&mut source)
        .await?;
    source.close().await?;
    let pool = open_pool_path(&path).await?;
    rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
    let (vehicle_id, epoch) = super::create_fixture(&pool, super::Scenario::PoweredVacuum).await?;
    let components =
        rocketcon_db::repositories::vehicle::list_components_for_vehicle(&pool, &vehicle_id)
            .await?;
    let at_epoch = Duration::new(super::INITIAL_AT_EPOCH);
    let cpu_id = components
        .iter()
        .find(|(_, record)| {
            matches!(
                record.details(),
                rocketcon_core::domain::ComponentDetails::Cpu
            )
        })
        .ok_or("CPU fixture missing")?
        .0
        .id();
    let irrelevant_state = rocketcon_core::domain::ComponentOperationalState::new_simple(
        cpu_id, 0.37, epoch, at_epoch,
    )?;
    rocketcon_db::repositories::operational_state::upsert(&pool, &irrelevant_state).await?;

    let mut reservoir_states = HashMap::new();
    let mut operational_states = HashMap::new();
    for (entry, _) in &components {
        if let Some(state) =
            rocketcon_db::repositories::energy_reservoir::get_by_vehicle_component_id(
                &pool,
                &entry.id(),
            )
            .await?
        {
            reservoir_states.insert(entry.id(), state);
        }
        if let Some(state) =
            rocketcon_db::repositories::operational_state::get_by_vehicle_component_id(
                &pool,
                &entry.id(),
            )
            .await?
        {
            operational_states.insert(entry.id(), state);
        }
    }
    let payload_masses =
        rocketcon_app::aeroespacial::resolve_payload_masses(&pool, &components, epoch, at_epoch)
            .await?;
    let physical_state =
        rocketcon_db::repositories::vehicle_physical_state::get_by_vehicle_id(&pool, &vehicle_id)
            .await?;

    for stages in [vec![0], vec![1]] {
        let expected = VehicleSnapshot::from_components(
            vehicle_id,
            &components,
            stages.clone(),
            &reservoir_states,
            &operational_states,
            1.0,
            &payload_masses,
            physical_state.clone(),
            epoch,
            at_epoch,
        )?;
        let actual = rocketcon_app::aeroespacial::resolve_vehicle_snapshot_with_components(
            &pool,
            vehicle_id,
            &components,
            Some(&stages),
            1.0,
            epoch,
            at_epoch,
        )
        .await?;
        assert_eq!(actual, expected);
    }

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
