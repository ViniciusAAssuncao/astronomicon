use astronomicon_core::units::Duration;
use rocketcon_db::save::{migrations::run_rocketcon_migrations, template::create_save_copy};
use rocketcon_sim::RocketconSession;
use sqlx::SqlitePool;
use std::error::Error;
use std::path::Path;
use uuid::Uuid;

const VEHICLE_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b01";
const MEROS_ID: &str = "60dc1b97-29a6-4b1c-95fd-42b229079928";
const PROPELLANT_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b02";
const ENGINE_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b03";
const TANK_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b04";
const BATTERY_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b05";
const CPU_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b06";
const RCS_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b07";
const ENTRY_BASE: u128 = 0x715c3661e69f475c8c7864927ae12000;
const AT_EPOCH: f64 = 1.0;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let destination = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "saves/flight-control-demo.db".to_string());
    let mode = std::env::args().nth(2);
    let eccentric = mode.as_deref() == Some("--eccentric");
    let fine_throttle = mode.as_deref() == Some("--fine-throttle") || eccentric;
    let throttleable = fine_throttle || mode.as_deref() == Some("--throttleable");
    let path = Path::new(&destination);
    create_fixture(path, throttleable, fine_throttle, eccentric).await?;
    println!("save: {}", path.canonicalize()?.display());
    println!("vehicle: {VEHICLE_ID}");
    Ok(())
}

async fn create_fixture(
    path: &Path, throttleable: bool, fine_throttle: bool, eccentric: bool,
) -> Result<(), Box<dyn Error>> {
    if path.exists() {
        return Err(format!("save already exists: {}", path.display()).into());
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    create_save_copy(path).await?;
    let pool = astronomicon_db::connection::open_pool_path(path).await?;
    run_rocketcon_migrations(&pool).await?;
    populate(&pool, throttleable, fine_throttle, eccentric).await?;
    let session = RocketconSession::load(path, Uuid::parse_str(VEHICLE_ID)?).await?;
    if session.vehicle_components().len() != 16 {
        return Err("test vehicle assembly is incomplete".into());
    }
    if eccentric {
        let preview = session.orbit_preview().await?;
        if preview.next_periapsis.is_none() || preview.next_apoapsis.is_none() {
            return Err("eccentric test orbit is missing an apsis marker".into());
        }
    }
    session.close().await;
    let (busy, _, _): (i64, i64, i64) = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
        .fetch_one(&pool)
        .await?;
    if busy != 0 {
        return Err("could not checkpoint test save".into());
    }
    pool.close().await;
    Ok(())
}

async fn populate(
    pool: &SqlitePool, throttleable: bool, fine_throttle: bool, eccentric: bool,
) -> Result<(), Box<dyn Error>> {
    let planet_id = Uuid::parse_str(MEROS_ID)?;
    let epoch = rocketcon_app::universe::resolve_universe_epoch(pool).await?;
    let environment = rocketcon_app::environment::load_environment_snapshot(
        pool,
        planet_id,
        epoch,
        Duration::new(AT_EPOCH),
    )
    .await?;
    let radius = environment
        .planet
        .equatorial_radius()
        .ok_or("planet has no radius")?
        .value();
    let orbital_radius = radius + 200_000.0;
    let mu = 6.67430e-11 * environment.planet.mass().value();
    let (_, planet_velocity) = rocketcon_app::orbital::soi::resolve_body_state_at_epoch(
        pool,
        planet_id,
        environment.system_id,
        epoch + Duration::new(AT_EPOCH),
    )
    .await?;
    let position = environment.planet_position.raw();
    let velocity = planet_velocity.raw();

    sqlx::query("INSERT INTO vehicles (id, name, vehicle_kind) VALUES (?, 'Rocketcon Attitude Test', 'Rocket')")
        .bind(VEHICLE_ID).execute(pool).await?;
    sqlx::query("INSERT INTO propellants (id, name, propellant_kind, density_kg_per_m3, is_cryogenic, is_hypergolic) VALUES (?, 'Test Monopropellant', 'LiquidFuel', 800, 0, 0)")
        .bind(PROPELLANT_ID).execute(pool).await?;

    component(
        pool,
        ENGINE_ID,
        "Engine",
        "Motor de teste",
        100.0,
        1.0,
        0.8,
        0.0,
    )
    .await?;
    text_attribute(pool, ENGINE_ID, "fuel_propellant_id", PROPELLANT_ID).await?;
    text_attribute(pool, ENGINE_ID, "ignition_type",
        if throttleable { "Restartable" } else { "SingleBurn" }).await?;
    numeric_attribute(pool, ENGINE_ID, "specific_impulse_vacuum_s", 300.0).await?;
    numeric_attribute(pool, ENGINE_ID, "max_thrust_n", if eccentric { 30_000.0 } else { 10_000.0 }).await?;
    if throttleable {
        numeric_attribute(pool, ENGINE_ID, "min_throttle_fraction",
            if fine_throttle { 0.01 } else { 0.25 }).await?;
    } else {
        numeric_attribute(pool, ENGINE_ID, "integral_propellant_mass_kg", 30.0).await?;
    }
    let engine_entry = mount(
        pool,
        1,
        ENGINE_ID,
        if throttleable { "Motor regulável" } else { "Motor principal" },
        0,
        [0.0, 0.0, -1.5],
        None,
    )
    .await?;
    sqlx::query("INSERT INTO component_operational_states (vehicle_component_id, load_fraction, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, 0, ?, ?)")
        .bind(engine_entry).bind(epoch.value()).bind(AT_EPOCH).execute(pool).await?;

    component(
        pool,
        TANK_ID,
        "PropellantTank",
        "Tanque de teste",
        20.0,
        1.0,
        0.9,
        0.0,
    )
    .await?;
    text_attribute(pool, TANK_ID, "propellant_id", PROPELLANT_ID).await?;
    numeric_attribute(pool, TANK_ID, "max_propellant_mass_kg",
        if fine_throttle { 100.0 } else { 30.0 }).await?;
    mount(
        pool,
        2,
        TANK_ID,
        "Tanque de propelente",
        0,
        [0.0, 0.0, -0.5],
        None,
    )
    .await?;

    component(
        pool,
        BATTERY_ID,
        "Battery",
        "Bateria de teste",
        50.0,
        0.7,
        0.8,
        0.0,
    )
    .await?;
    numeric_attribute(pool, BATTERY_ID, "capacity_j", 100_000.0).await?;
    numeric_attribute(pool, BATTERY_ID, "max_discharge_power_w", 10_000.0).await?;
    numeric_attribute(pool, BATTERY_ID, "max_charge_power_w", 2_000.0).await?;
    let battery_entry = mount(pool, 3, BATTERY_ID, "Bateria", 0, [0.0, 0.0, 0.5], None).await?;
    sqlx::query("INSERT INTO energy_reservoir_states (vehicle_component_id, stored_energy_j, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, 90000, ?, ?)")
        .bind(battery_entry).bind(epoch.value()).bind(AT_EPOCH).execute(pool).await?;

    component(
        pool,
        CPU_ID,
        "Cpu",
        "Computador de voo",
        100.0,
        0.8,
        0.8,
        5.0,
    )
    .await?;
    mount(
        pool,
        4,
        CPU_ID,
        "Computador de voo",
        1,
        [0.0, 0.0, 1.5],
        None,
    )
    .await?;

    component(
        pool,
        RCS_ID,
        "ReactionControlThruster",
        "Propulsor RCS",
        2.0,
        0.2,
        0.2,
        0.0,
    )
    .await?;
    text_attribute(pool, RCS_ID, "propellant_id", PROPELLANT_ID).await?;
    numeric_attribute(pool, RCS_ID, "specific_impulse_vacuum_s", 220.0).await?;
    numeric_attribute(pool, RCS_ID, "max_thrust_n", 100.0).await?;
    let thrusters = [
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -1.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0]),
        ([0.0, 0.0, 1.0], [-1.0, 0.0, 0.0]),
        ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    for (index, (offset, axis)) in thrusters.into_iter().enumerate() {
        let label = format!("RCS {:02}", index + 1);
        mount(
            pool,
            10 + index as u128,
            RCS_ID,
            &label,
            0,
            offset,
            Some(axis),
        )
        .await?;
    }

    sqlx::query("INSERT INTO vehicle_physical_states (vehicle_id, position_x_m, position_y_m, position_z_m, velocity_x_m_s, velocity_y_m_s, velocity_z_m_s, orientation_q_w, orientation_q_x, orientation_q_y, orientation_q_z, angular_velocity_x_rad_s, angular_velocity_y_rad_s, angular_velocity_z_rad_s, reference_body_id, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, ?, ?, ?)")
        .bind(VEHICLE_ID).bind(position.0 + orbital_radius).bind(position.1).bind(position.2)
        .bind(velocity.0).bind(velocity.1 + (mu / orbital_radius).sqrt() * if eccentric { 1.05 } else { 1.0 }).bind(velocity.2)
        .bind(if eccentric { std::f64::consts::FRAC_1_SQRT_2 } else { 1.0 })
        .bind(if eccentric { -std::f64::consts::FRAC_1_SQRT_2 } else { 0.0 })
        .bind(MEROS_ID).bind(epoch.value()).bind(AT_EPOCH).execute(pool).await?;
    Ok(())
}

async fn component(
    pool: &SqlitePool,
    id: &str,
    kind: &str,
    name: &str,
    mass: f64,
    length: f64,
    diameter: f64,
    power: f64,
) -> Result<(), Box<dyn Error>> {
    sqlx::query("INSERT INTO components (id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w) VALUES (?, ?, ?, ?, ?, ?, ?)")
        .bind(id).bind(name).bind(kind).bind(mass).bind(length).bind(diameter).bind(power)
        .execute(pool).await?;
    Ok(())
}

async fn numeric_attribute(
    pool: &SqlitePool,
    id: &str,
    key: &str,
    value: f64,
) -> Result<(), Box<dyn Error>> {
    sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, numeric_value) VALUES (?, ?, ?)")
        .bind(id).bind(key).bind(value).execute(pool).await?;
    Ok(())
}

async fn text_attribute(
    pool: &SqlitePool,
    id: &str,
    key: &str,
    value: &str,
) -> Result<(), Box<dyn Error>> {
    sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, text_value) VALUES (?, ?, ?)")
        .bind(id).bind(key).bind(value).execute(pool).await?;
    Ok(())
}

async fn mount(
    pool: &SqlitePool,
    index: u128,
    component_id: &str,
    label: &str,
    stage: i64,
    offset: [f64; 3],
    axis: Option<[f64; 3]>,
) -> Result<String, Box<dyn Error>> {
    let id = Uuid::from_u128(ENTRY_BASE + index).to_string();
    let axis = axis.unwrap_or([0.0, 0.0, 0.0]);
    sqlx::query("INSERT INTO vehicle_components (id, vehicle_id, component_id, instance_label, stage_index, mount_offset_x_m, mount_offset_y_m, mount_offset_z_m, actuation_axis_x, actuation_axis_y, actuation_axis_z) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id).bind(VEHICLE_ID).bind(component_id).bind(label).bind(stage)
        .bind(offset[0]).bind(offset[1]).bind(offset[2])
        .bind(if axis == [0.0; 3] { None } else { Some(axis[0]) })
        .bind(if axis == [0.0; 3] { None } else { Some(axis[1]) })
        .bind(if axis == [0.0; 3] { None } else { Some(axis[2]) })
        .execute(pool).await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unpowered_orbit_has_continuous_telemetry() -> Result<(), Box<dyn Error>> {
        std::env::set_current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
        let path = Path::new("target").join(format!("flight-coast-{}.db", Uuid::new_v4()));
        create_fixture(&path, true, true, false).await?;
        let mut session = RocketconSession::load(&path, Uuid::parse_str(VEHICLE_ID)?).await?;
        let mut previous = session.snapshot().clone();
        assert!(previous.main_engine_loads.iter().all(|(_, load)| *load == 0.0));

        for _ in 0..50 {
            let current = session.step(0.02).await?.clone();
            let altitude_change = current.altitude_m.unwrap() - previous.altitude_m.unwrap();
            let vertical_speed = current.reference_vertical_speed_m_s.unwrap();
            let previous_vertical_speed = previous.reference_vertical_speed_m_s.unwrap();
            let relative_speed = current.reference_speed_m_s.unwrap();
            let previous_relative_speed = previous.reference_speed_m_s.unwrap();
            assert!(current.total_g_load.unwrap() < 0.05);
            assert!((vertical_speed - previous_vertical_speed).abs() < 1.0);
            assert!((relative_speed - previous_relative_speed).abs() < 1.0);
            assert!((altitude_change - vertical_speed * 0.02).abs() < 1.0);
            previous = current;
        }

        session.close().await;
        std::fs::remove_file(path)?;
        Ok(())
    }
}
