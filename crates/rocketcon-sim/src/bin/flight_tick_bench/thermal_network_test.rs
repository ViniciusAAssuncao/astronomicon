use astronomicon_core::units::Duration;
use astronomicon_db::connection::open_pool_path;
use rocketcon_app::thermal::network_assembly::assemble_vehicle_thermal_network;
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_db::repositories::vehicle::list_components_for_vehicle;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

#[tokio::test]
async fn shared_material_override_reaches_every_vehicle_instance() -> Result<(), Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!("rocketcon-thermal-{}.db", Uuid::new_v4()));
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

    let material_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO materials (id, name, material_class, density_kg_per_m3, \
         specific_heat_capacity_j_per_kg_k, thermal_conductivity_w_per_m_k, \
         thermal_expansion_coefficient_per_k, melting_point_k, max_service_temperature_k, \
         youngs_modulus_pa, base_yield_strength_pa, base_ultimate_tensile_strength_pa, \
         emissivity, solar_absorptivity) \
         VALUES (?, 'Thermal fixture metal', 'Metal', 2700, 900, 200, 2.3e-5, 933, 600, \
         6.9e10, 2.5e8, 3e8, 0.8, 0.7)",
    )
    .bind(material_id.to_string())
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO component_attributes (component_id, attribute_key, text_value) VALUES (?, 'material_id', ?)",
    )
    .bind(super::COMPONENT_ID)
    .bind(material_id.to_string())
    .execute(&pool)
    .await?;

    let components = list_components_for_vehicle(&pool, &vehicle_id).await?;
    let network = assemble_vehicle_thermal_network(
        &pool,
        vehicle_id,
        &components,
        &[0],
        &HashMap::new(),
        None,
        None,
        None,
        None,
    )
    .await?;

    let component_id = Uuid::parse_str(super::COMPONENT_ID)?;
    let instances: Vec<_> = network
        .nodes
        .iter()
        .filter(|node| node.component_id == component_id)
        .collect();
    assert_eq!(instances.len(), 8);
    assert!(
        instances
            .iter()
            .all(|node| node.material_id == Some(material_id))
    );
    assert_eq!(
        instances
            .iter()
            .map(|node| node.vehicle_component_id)
            .collect::<HashSet<_>>()
            .len(),
        instances.len()
    );

    rocketcon_app::aeroespacial::advance_vehicle_simulation(
        &pool,
        vehicle_id,
        Duration::new(super::DT),
        epoch,
        &VehicleControlInput::new(),
    )
    .await?;
    let persisted_nodes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM thermal_node_states WHERE captured_at_epoch_s > ?",
    )
    .bind(super::INITIAL_AT_EPOCH)
    .fetch_one(&pool)
    .await?;
    assert_eq!(persisted_nodes as usize, network.nodes.len());

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
