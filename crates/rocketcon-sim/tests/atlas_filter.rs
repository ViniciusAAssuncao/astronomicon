use rocketcon_sim::ForecastReader;
use std::time::Instant;
use uuid::Uuid;

#[tokio::test]
async fn atlas_hides_small_objects_before_sampling_orbits() {
    let Ok(path) = std::env::var("ROCKETCON_ATLAS_FILTER_TEST_SAVE") else {
        return;
    };
    let vehicle = Uuid::parse_str("715c3661-e69f-475c-8c78-64927ae12b01").unwrap();
    let reader = ForecastReader::open(path, vehicle).await.unwrap();
    let start = Instant::now();
    let filtered = reader.celestial_atlas_filtered(false).await.unwrap();
    let filtered_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let complete = reader.celestial_atlas_filtered(true).await.unwrap();
    let complete_ms = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(complete.bodies.len(), filtered.bodies.len() + 401);
    for body in &filtered.bodies {
        assert!(!body.name.starts_with("Pedregulho"));
        assert_ne!(body.name, "Small-edge");
    }
    for name in ["Ceres-like", "Almost-dwarf"] {
        let body = filtered
            .bodies
            .iter()
            .find(|body| body.name == name)
            .unwrap();
        assert_eq!(body.kind, 4);
        assert!(body.orbit_points_m.len() > 2);
    }
    assert!(complete.bodies.iter().any(|body| body.name == "Small-edge"));
    assert_eq!(filtered.reference_body_id, complete.reference_body_id);
    assert_eq!(filtered.epoch_seconds, complete.epoch_seconds);
    assert!(
        filtered
            .bodies
            .iter()
            .any(|body| body.id == filtered.reference_body_id)
    );
    println!(
        "ATLAS_FILTER_PASS default={} all={} filtered_ms={filtered_ms:.2} all_ms={complete_ms:.2}",
        filtered.bodies.len(),
        complete.bodies.len()
    );
    reader.close().await;
}
