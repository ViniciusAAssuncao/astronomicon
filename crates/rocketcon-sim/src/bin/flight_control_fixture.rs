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
    let path = Path::new(&destination);
    create_fixture(path).await?;
    println!("save: {}", path.canonicalize()?.display());
    println!("vehicle: {VEHICLE_ID}");
    Ok(())
}

async fn create_fixture(path: &Path) -> Result<(), Box<dyn Error>> {
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
    populate(&pool).await?;
    let session = RocketconSession::load(path, Uuid::parse_str(VEHICLE_ID)?).await?;
    if session.vehicle_components().len() != 16 {
        return Err("test vehicle assembly is incomplete".into());
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

async fn populate(pool: &SqlitePool) -> Result<(), Box<dyn Error>> {
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
    text_attribute(pool, ENGINE_ID, "ignition_type", "SingleBurn").await?;
    numeric_attribute(pool, ENGINE_ID, "specific_impulse_vacuum_s", 300.0).await?;
    numeric_attribute(pool, ENGINE_ID, "max_thrust_n", 10_000.0).await?;
    numeric_attribute(pool, ENGINE_ID, "integral_propellant_mass_kg", 30.0).await?;
    mount(
        pool,
        1,
        ENGINE_ID,
        "Motor inativo",
        0,
        [0.0, 0.0, -1.5],
        None,
    )
    .await?;

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
    numeric_attribute(pool, TANK_ID, "max_propellant_mass_kg", 30.0).await?;
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

    sqlx::query("INSERT INTO vehicle_physical_states (vehicle_id, position_x_m, position_y_m, position_z_m, velocity_x_m_s, velocity_y_m_s, velocity_z_m_s, orientation_q_w, orientation_q_x, orientation_q_y, orientation_q_z, angular_velocity_x_rad_s, angular_velocity_y_rad_s, angular_velocity_z_rad_s, reference_body_id, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, ?, ?, ?, ?, ?, ?, 1, 0, 0, 0, 0, 0, 0, ?, ?, ?)")
        .bind(VEHICLE_ID).bind(position.0 + orbital_radius).bind(position.1).bind(position.2)
        .bind(velocity.0).bind(velocity.1 + (mu / orbital_radius).sqrt()).bind(velocity.2)
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
    use rocketcon_core::domain::VehicleControlInput;

    #[tokio::test]
    async fn attitude_release_enters_coast_and_accepts_new_axis() -> Result<(), Box<dyn Error>> {
        let previous_directory = std::env::current_dir()?;
        std::env::set_current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
        let path = std::env::temp_dir().join(format!("rocketcon-attitude-{}.db", Uuid::new_v4()));
        create_fixture(&path).await?;
        let mut session = RocketconSession::load(&path, Uuid::parse_str(VEHICLE_ID)?).await?;
        session.apply_control(VehicleControlInput::new().with_pitch_yaw_roll(1.0, 0.0, 0.0));
        session.step(0.02).await?;
        let pitch = session.step(0.02).await?.angular_velocity_rad_s[0];
        assert!(pitch > 0.0);
        session.apply_control(VehicleControlInput::new().with_pitch_yaw_roll(0.0, 0.0, 0.0));
        let coast = session.step(0.02).await?;
        assert!(coast.angular_velocity_rad_s[0].is_finite());
        assert!(coast.altitude_m.is_some());
        session.apply_control(VehicleControlInput::new().with_pitch_yaw_roll(0.0, 0.0, -1.0));
        let roll = session.step(0.02).await?;
        assert!(roll.angular_velocity_rad_s[2] < 0.0);
        session.close().await;
        std::fs::remove_file(path)?;
        std::env::set_current_dir(previous_directory)?;
        Ok(())
    }
}
