use super::*;
use rocketcon_app::orbital::hybrid_regime::choose_regime;
use rocketcon_db::repositories::vehicle_physical_state;
use std::path::PathBuf;

#[tokio::test]
#[ignore = "requires local C15 mission fixture"]
async fn two_day_windows_preserve_state_and_progress_monotonicity() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../saves/c15-multiarc-inbound.db");
    assert!(path.is_file(), "C15 mission fixture unavailable");
    let pool = astronomicon_db::connection::open_read_only_pool_path(&path)
        .await
        .unwrap();
    let vehicle_id = Uuid::parse_str("715c3661-e69f-475c-8c78-64927ae12b01").unwrap();
    let state = vehicle_physical_state::get_by_vehicle_id(&pool, &vehicle_id)
        .await
        .unwrap()
        .unwrap();
    let epoch = rocketcon_app::universe::resolve_universe_epoch(&pool)
        .await
        .unwrap();
    let environment = rocketcon_app::environment::load_environment_snapshot(
        &pool,
        state.reference_body_id(),
        epoch,
        state.captured_at_epoch(),
    )
    .await
    .unwrap();
    let catalog = EphemerisCatalog::load(&pool, environment.system_id)
        .await
        .unwrap();
    let mut sources = match choose_regime(
        &catalog,
        &state,
        Duration::new(MAX_BOUND_HORIZON),
        false,
        RegimePolicy::forecast(),
    )
    .unwrap()
    {
        GravityRegime::Restricted { sources } => sources,
        _ => catalog
            .gravitating_bodies()
            .into_iter()
            .map(|(id, _)| id)
            .collect(),
    };
    sources.sort();
    let mut snapshots = Vec::new();
    let requested = MAX_BOUND_HORIZON * 2.2;
    let result = sample_restricted_forecast(
        &catalog,
        state.captured_total_epoch(),
        state.reference_body_id(),
        state.position(),
        state.velocity(),
        requested,
        &sources,
        &mut || false,
        &mut |partial| {
            let arc = partial.arcs.last().unwrap();
            snapshots.push((partial.elapsed_seconds, arc.end_seconds));
        },
    )
    .unwrap();
    assert!(snapshots.len() >= 2, "only one forecast window produced");
    for pair in snapshots.windows(2) {
        assert!(pair[1].0 > pair[0].0);
        assert!(pair[1].1 > pair[0].1);
    }
    assert!(snapshots[0].0 >= MAX_BOUND_HORIZON - 1.0);
    assert_eq!(result.elapsed_seconds, snapshots.last().unwrap().0);
    for arcs in result.arcs.windows(2) {
        let a = arcs[0].points_m.last().unwrap();
        let b = arcs[1].points_m.first().unwrap();
        let delta = a
            .iter()
            .zip(b)
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(delta <= 1.0, "SOI arc gap {delta} m");
    }
    println!(
        "C15_MULTIWINDOW windows={} reached_s={:.3} target_s={requested:.3}",
        snapshots.len(),
        result.elapsed_seconds
    );
    pool.close().await;
}
