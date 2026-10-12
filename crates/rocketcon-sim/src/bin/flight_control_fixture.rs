#[path = "flight_control_fixture/soi_scenarios.rs"]
mod soi_scenarios;

use astronomicon_core::units::Duration;
use rocketcon_db::save::{migrations::run_rocketcon_migrations, template::create_save_copy};
use rocketcon_sim::{RocketconSession, planet_equatorial_reference, OrbitalReferenceFrame};
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
    let scenario = mode.as_deref().filter(|value|
        value.starts_with("--node-") || value.starts_with("--c8-") ||
        value.starts_with("--c9-") || value.starts_with("--c10-"));
    if let Some(name) = scenario {
        if !matches!(name, "--node-inclined" | "--node-polar" | "--node-retrograde" |
            "--node-equatorial" | "--node-escape" | "--node-separated" |
            "--c8-meros-equatorial" | "--c8-meros-inclined" |
            "--c9-soi-outbound" | "--c9-soi-inbound" | "--c9-soi-inbound-stable" | "--c9-soi-inbound-nearcircular" | "--c9-soi-outside" |
            "--c10-soi-outbound" | "--c10-soi-inbound" | "--c10-soi-tangent") {
            return Err(format!("unknown orbital scenario: {name}").into());
        }
    }
    let path = Path::new(&destination);
    create_fixture(path, throttleable, fine_throttle, eccentric, scenario).await?;
    println!("save: {}", path.canonicalize()?.display());
    println!("vehicle: {VEHICLE_ID}");
    Ok(())
}

async fn create_fixture(
    path: &Path, throttleable: bool, fine_throttle: bool, eccentric: bool,
    scenario: Option<&str>,
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
    populate(&pool, throttleable, fine_throttle, eccentric, scenario).await?;
    let session = RocketconSession::load(path, Uuid::parse_str(VEHICLE_ID)?).await?;
    if session.vehicle_components().len() != 16 {
        return Err("test vehicle assembly is incomplete".into());
    }
    if eccentric || scenario.is_some() {
        let preview = session.orbit_preview().await?;
        if eccentric && (preview.next_periapsis.is_none() || preview.next_apoapsis.is_none()) {
            return Err("eccentric test orbit is missing an apsis marker".into());
        }
        if let Some(name) = scenario {
            let c8 = name.starts_with("--c8-");
            let c9 = name.starts_with("--c9-");
            let c10 = name.starts_with("--c10-");
            let equatorial = name == "--node-equatorial";
            let escape = name == "--node-escape";
            if !(c8 || c9 || c10) && (preview.nodes.inclination_rad.is_none() ||
                preview.nodes.ascending_longitude_rad.is_some() == equatorial ||
                preview.nodes.next_ascending.is_some() == equatorial ||
                (!equatorial && !escape && preview.nodes.next_descending.is_none()) ||
                (escape && preview.nodes.next_descending.is_some())) {
                return Err(format!("nodal fixture preview is invalid: {name} {:#?}", preview.nodes).into());
            }
            if name == "--node-separated" {
                let peri = preview.next_periapsis.as_ref().ok_or("missing periapsis")?;
                let apo = preview.next_apoapsis.as_ref().ok_or("missing apoapsis")?;
                let asc = preview.nodes.next_ascending.as_ref().ok_or("missing ascending node")?;
                let desc = preview.nodes.next_descending.as_ref().ok_or("missing descending node")?;
                for (apsis, node) in [(peri, asc), (peri, desc), (apo, asc), (apo, desc)] {
                    let separation = apsis.relative_position_m.iter()
                        .zip(node.relative_position_m.iter())
                        .map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
                    if separation < 100_000.0 {
                        return Err("separated orbit has coincident events".into());
                    }
                }
            }
            if c8 {
                let eq = preview.equatorial.as_ref().ok_or("planet has no equatorial reference")?;
                let inclination = eq.nodes.inclination_rad.ok_or("equatorial inclination missing")?.to_degrees();
                let expected: f64 = if name == "--c8-meros-equatorial" { 0.0 } else { 35.0 };
                if (inclination - expected).abs() > 1e-5 {
                    return Err(format!("wrong equatorial inclination: {inclination}").into());
                }
                if name == "--c8-meros-equatorial" && eq.nodes.ascending_longitude_rad.is_some() {
                    return Err("equatorial orbit has an invented longitude".into());
                }
                if name == "--c8-meros-inclined" &&
                    (eq.nodes.next_ascending.is_none() || eq.nodes.next_descending.is_none()) {
                    return Err("inclined equatorial fixture missing node".into());
                }
                println!("equatorial_inclination_deg: {inclination:.6}");
                println!("equatorial_next_ascending_s: {:?}", eq.nodes.next_ascending.as_ref().map(|n| n.time_to_seconds));
                println!("equatorial_next_descending_s: {:?}", eq.nodes.next_descending.as_ref().map(|n| n.time_to_seconds));
            }
            if c9 || name == "--c10-soi-outbound" {
                let soi = preview.soi.as_ref().ok_or("SOI missing")?;
                let expected_outside = name == "--c9-soi-outside";
                let expected_exit = name == "--c9-soi-outbound" ||
                    name == "--c9-soi-inbound" || name == "--c10-soi-outbound";
                if soi.outside != expected_outside || soi.exit.is_some() != expected_exit {
                    return Err(format!("C9 SOI scenario invalid: {name} {soi:?}").into());
                }
                if let Some(exit) = &soi.exit {
                    println!("soi_exit_seconds: {:.6}", exit.time_to_seconds);
                    println!("soi_exit_distance_m: {:.3}",
                        exit.relative_position_m.iter().map(|v| v*v).sum::<f64>().sqrt());
                }
                println!("soi_radius_m: {:.3} outside: {}", soi.radius_m, soi.outside);
                println!("preview_target_seconds: {:.3} rendered_seconds: {:.3} points: {} end: {:?}",
                    preview.horizon_seconds, preview.rendered_horizon_seconds,
                    preview.relative_points_m.len(), preview.end_reason);
                println!("next_periapsis_s: {:?}", preview.next_periapsis.as_ref().map(|p| p.time_to_seconds));
            }
            if matches!(name, "--c10-soi-inbound" | "--c10-soi-tangent") && preview.soi.is_some() {
                return Err("stellar-reference fixture has a planetary SOI".into());
            }
            println!("scenario: {name}");
            println!("inclination_deg: {:.4}", preview.nodes.inclination_rad.unwrap().to_degrees());
            println!("ascending_longitude_deg: {:?}", preview.nodes.ascending_longitude_rad.map(f64::to_degrees));
            println!("next_ascending_s: {:?}", preview.nodes.next_ascending.as_ref().map(|n| n.time_to_seconds));
            println!("next_descending_s: {:?}", preview.nodes.next_descending.as_ref().map(|n| n.time_to_seconds));
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
    scenario: Option<&str>,
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
    let planet = environment.planet.as_ref().ok_or("expected a planet in this scenario")?;
    let radius = planet
        .equatorial_radius()
        .ok_or("planet has no radius")?
        .value();
    let orbital_radius = radius + 200_000.0;
    let mu = 6.67430e-11 * planet.mass().value();
    let (_, planet_velocity) = rocketcon_app::orbital::soi::resolve_body_state_at_epoch(
        pool,
        planet_id,
        environment.system_id,
        epoch + Duration::new(AT_EPOCH),
    )
    .await?;
    let position = environment.reference_position().raw();
    let velocity = planet_velocity.raw();

    sqlx::query("INSERT INTO vehicles (id, name, vehicle_kind) VALUES (?, ?, 'Rocket')")
        .bind(VEHICLE_ID).bind(scenario.unwrap_or("Rocketcon Attitude Test")).execute(pool).await?;
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

    let (offset, relative_velocity) = scenario.map(|name| {
        if name.starts_with("--c9-") || name.starts_with("--c10-") {
            soi_scenarios::soi_orbit_state(name, planet, &environment.star)
        } else if name.starts_with("--c8-") {
            equatorial_orbit_state(name, radius, mu,
                planet_equatorial_reference(planet)
                    .expect("Meridian test planet must have a physical equator"))
        } else {
            nodal_orbit_state(name, radius, mu)
        }
    }).unwrap_or((
            [orbital_radius, 0.0, 0.0],
            [0.0, (mu / orbital_radius).sqrt() * if eccentric { 1.05 } else { 1.0 }, 0.0]));
    sqlx::query("INSERT INTO vehicle_physical_states (vehicle_id, position_x_m, position_y_m, position_z_m, velocity_x_m_s, velocity_y_m_s, velocity_z_m_s, orientation_q_w, orientation_q_x, orientation_q_y, orientation_q_z, angular_velocity_x_rad_s, angular_velocity_y_rad_s, angular_velocity_z_rad_s, reference_body_id, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, ?, ?, ?)")
        .bind(VEHICLE_ID).bind(position.0 + offset[0]).bind(position.1 + offset[1]).bind(position.2 + offset[2])
        .bind(velocity.0 + relative_velocity[0]).bind(velocity.1 + relative_velocity[1])
        .bind(velocity.2 + relative_velocity[2])
        .bind(if eccentric { std::f64::consts::FRAC_1_SQRT_2 } else { 1.0 })
        .bind(if eccentric { -std::f64::consts::FRAC_1_SQRT_2 } else { 0.0 })
        .bind(if matches!(scenario, Some("--c10-soi-inbound" | "--c10-soi-tangent")) {
            environment.star.id().to_string()
        } else { MEROS_ID.to_string() })
        .bind(epoch.value()).bind(AT_EPOCH).execute(pool).await?;
    Ok(())
}

fn equatorial_orbit_state(
    scenario: &str,
    body_radius: f64,
    mu: f64,
    frame: OrbitalReferenceFrame,
) -> ([f64; 3], [f64; 3]) {
    let (inclination_deg, longitude_deg, eccentricity, argument_deg) = match scenario {
        "--c8-meros-equatorial" => (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64),
        "--c8-meros-inclined" => (35.0, 42.0, 0.18, 50.0),
        _ => unreachable!(),
    };
    let inclination = inclination_deg.to_radians();
    let longitude = longitude_deg.to_radians();
    let anomaly = -45.0_f64.to_radians();
    let argument = argument_deg.to_radians();
    let phase = anomaly + argument;
    let node = frame.horizontal * longitude.cos() + frame.vertical * longitude.sin();
    let transverse = frame.normal.cross(&node) * inclination.cos() +
        frame.normal * inclination.sin();
    let radial = node * phase.cos() + transverse * phase.sin();
    let tangent = node * -phase.sin() + transverse * phase.cos();
    let radius = body_radius + 520_000.0;
    let p = radius * (1.0 + eccentricity * anomaly.cos());
    let speed = (mu / p).sqrt();
    let v = radial * (speed * eccentricity * anomaly.sin()) +
        tangent * (speed * (1.0 + eccentricity * anomaly.cos()));
    ([radial.0 * radius, radial.1 * radius, radial.2 * radius], [v.0, v.1, v.2])
}

fn nodal_orbit_state(scenario: &str, body_radius: f64, mu: f64) -> ([f64; 3], [f64; 3]) {
    let (inclination, longitude, eccentricity, anomaly, altitude, periapsis_argument) = match scenario {
        "--node-inclined" => (28.5_f64, 35.0_f64, 0.15, -45.0_f64, 500_000.0, 0.0),
        "--node-separated" => (28.5, 35.0, 0.15, -45.0, 500_000.0, 60.0),
        "--node-polar" => (90.0, 0.0, 0.0, -45.0, 450_000.0, 0.0),
        "--node-retrograde" => (145.0, 72.0, 0.10, -45.0, 500_000.0, 0.0),
        "--node-equatorial" => (0.0, 0.0, 0.0, -45.0, 450_000.0, 0.0),
        "--node-escape" => (60.0, 50.0, 1.50, -15.0, 1_000_000.0, 0.0),
        _ => unreachable!(),
    };
    let inclination = inclination.to_radians();
    let longitude = longitude.to_radians();
    let anomaly = anomaly.to_radians();
    let periapsis_argument: f64 = periapsis_argument;
    let orbital_angle = anomaly + periapsis_argument.to_radians();
    let node = [longitude.cos(), longitude.sin(), 0.0];
    let transverse = [
        -longitude.sin() * inclination.cos(),
        longitude.cos() * inclination.cos(),
        inclination.sin(),
    ];
    let radial = std::array::from_fn::<_, 3, _>(|axis|
        node[axis] * orbital_angle.cos() + transverse[axis] * orbital_angle.sin());
    let tangent = std::array::from_fn::<_, 3, _>(|axis|
        -node[axis] * orbital_angle.sin() + transverse[axis] * orbital_angle.cos());
    let radius = body_radius + altitude;
    let p = radius * (1.0 + eccentricity * anomaly.cos());
    let scale = (mu / p).sqrt();
    let velocity = std::array::from_fn(|axis| scale *
        (eccentricity * anomaly.sin() * radial[axis] +
            (1.0 + eccentricity * anomaly.cos()) * tangent[axis]));
    (radial.map(|axis| axis * radius), velocity)
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