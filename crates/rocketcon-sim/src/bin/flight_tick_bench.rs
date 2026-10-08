use astronomicon_core::units::Duration;
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_db::tick_transaction::TickTransactionSession;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::error::Error;
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;
use uuid::Uuid;

#[path = "flight_tick_bench/aero_run.rs"]
mod aero_run;
#[path = "flight_tick_bench/sql_trace.rs"]
mod sql_trace;
#[path = "flight_tick_bench/tick_run.rs"]
mod tick_run;

use tick_run::run_tick;

#[cfg(test)]
#[path = "flight_tick_bench/climate_snapshot_test.rs"]
mod climate_snapshot_test;

#[cfg(test)]
#[path = "flight_tick_bench/tick_atomicity_test.rs"]
mod tick_atomicity_test;

#[cfg(test)]
#[path = "flight_tick_bench/session_test.rs"]
mod session_test;

const VEHICLE_ID: &str = "e2897c6a-7d04-4ebc-882c-87984a74a200";
const COMPONENT_ID: &str = "56c1b894-4718-4f0c-94a0-bc22a6053180";
const BATTERY_ID: &str = "56c1b894-4718-4f0c-94a0-bc22a6053181";
const SOLAR_ID: &str = "56c1b894-4718-4f0c-94a0-bc22a6053182";
const ENGINE_ID: &str = "56c1b894-4718-4f0c-94a0-bc22a6053183";
const PROPELLANT_ID: &str = "56c1b894-4718-4f0c-94a0-bc22a6053184";
const MEROS_ID: &str = "60dc1b97-29a6-4b1c-95fd-42b229079928";
const HADAB_ID: &str = "4beb55b2-62de-4ec2-abe5-ec00290407f8";
const DT: f64 = 0.02;
const INITIAL_AT_EPOCH: f64 = 1.0;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Coast,
    Atmosphere,
    Powered,
}

impl Scenario {
    fn name(self) -> &'static str {
        match self {
            Self::Coast => "coast",
            Self::Atmosphere => "atmosphere",
            Self::Powered => "powered",
        }
    }

    fn planet_id(self) -> &'static str {
        match self {
            Self::Coast => MEROS_ID,
            Self::Atmosphere | Self::Powered => HADAB_ID,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ticks: usize = std::env::args()
        .nth(1)
        .map(|n| n.parse())
        .transpose()?
        .unwrap_or(20);
    if ticks == 0 {
        return Err("tick count must be positive".into());
    }
    let mode = std::env::var("ROCKETCON_PROFILE").unwrap_or_default();
    let aero_only = matches!(mode.as_str(), "aero-stages" | "aero-sql");
    let profiling = matches!(mode.as_str(), "stages" | "sql");
    let sql_count = matches!(mode.as_str(), "sql" | "aero-sql");

    for scenario in [Scenario::Coast, Scenario::Atmosphere, Scenario::Powered] {
        if aero_only && matches!(scenario, Scenario::Coast) {
            continue;
        }
        run_scenario(scenario, ticks, profiling, sql_count, aero_only).await?;
    }
    Ok(())
}

async fn run_scenario(
    scenario: Scenario,
    ticks: usize,
    profiling: bool,
    sql_count: bool,
    aero_only: bool,
) -> Result<(), Box<dyn Error>> {
    let path = format!("saves/bench-{}.db", scenario.name());
    if Path::new(&path).exists() {
        std::fs::remove_file(&path)?;
    }
    std::fs::create_dir_all("saves")?;
    let setup_start = Instant::now();
    rocketcon_db::save::template::create_save_copy(Path::new(&path)).await?;
    let url = format!("sqlite://{path}");
    let pool = astronomicon_db::connection::open_pool(&url).await?;
    rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
    let (vehicle_id, epoch) = create_fixture(&pool, scenario).await?;
    let pool = if sql_count {
        pool.close().await;
        let options = SqliteConnectOptions::from_str(&url)?
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .pragma("busy_timeout", "5000")
            .pragma("temp_store", "MEMORY")
            .pragma("cache_size", "-20000");
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?
    } else {
        pool
    };
    if sql_count {
        let mut connection = pool.acquire().await?;
        sql_trace::install(&mut connection).await?;
    }
    let setup_ms = setup_start.elapsed().as_secs_f64() * 1_000.0;
    if aero_only {
        aero_run::run(
            &pool,
            vehicle_id,
            Uuid::parse_str(scenario.planet_id())?,
            epoch,
            ticks,
            sql_count,
            setup_ms,
        )
        .await?;
        pool.close().await;
        return Ok(());
    }

    let tick_session = TickTransactionSession::new(&pool).await?;

    let input = VehicleControlInput::new();
    if sql_count {
        sql_trace::enable();
    }
    let sql_before = sql_trace::snapshot();
    let first_start = Instant::now();
    let (first, first_profile) = run_tick(
        &tick_session,
        vehicle_id,
        epoch,
        &input,
        profiling,
        sql_count,
    )
    .await?;
    let first_ms = first_start.elapsed().as_secs_f64() * 1_000.0;
    let sql_after_first = sql_trace::snapshot();
    validate(&first, INITIAL_AT_EPOCH + DT)?;
    let mut aerodynamic_ticks = usize::from(first.aerodynamics().is_some());
    let mut contact_ticks = usize::from(first.has_contact());
    let mut last_position = first.physical_state().position().raw();
    let mut final_speed = first.physical_state().velocity().raw().magnitude();
    let mut last_aero_tick = first.aerodynamics().map(|_| 1).unwrap_or(0);
    let mut last_aero_altitude = first.aerodynamics().map(|a| a.altitude.value());
    let mut stage_times: Vec<_> = first_profile
        .as_ref()
        .map(|profile| {
            profile
                .stages()
                .iter()
                .map(|stage| (stage.name, Vec::<f64>::new()))
                .collect()
        })
        .unwrap_or_default();

    let mut times = Vec::with_capacity(ticks.saturating_sub(1));
    for index in 1..ticks {
        let start = Instant::now();
        let (report, profile) = run_tick(
            &tick_session,
            vehicle_id,
            epoch,
            &input,
            profiling,
            sql_count,
        )
        .await?;
        times.push(start.elapsed().as_secs_f64() * 1_000.0);
        if let Some(profile) = profile {
            for (samples, stage) in stage_times.iter_mut().zip(profile.stages()) {
                samples.1.push(stage.duration.as_secs_f64() * 1_000.0);
            }
        }
        validate(&report, INITIAL_AT_EPOCH + (index + 1) as f64 * DT)?;
        aerodynamic_ticks += usize::from(report.aerodynamics().is_some());
        contact_ticks += usize::from(report.has_contact());
        last_position = report.physical_state().position().raw();
        final_speed = report.physical_state().velocity().raw().magnitude();
        if let Some(aerodynamics) = report.aerodynamics() {
            last_aero_tick = index + 1;
            last_aero_altitude = Some(aerodynamics.altitude.value());
        }
    }
    let sql_after_all = sql_trace::snapshot();
    sql_trace::disable();
    let persisted =
        rocketcon_db::repositories::vehicle_physical_state::get_by_vehicle_id(&pool, &vehicle_id)
            .await?
            .ok_or("missing persisted vehicle state")?;
    let final_epoch = persisted.captured_at_epoch().value();
    if (final_epoch - (INITIAL_AT_EPOCH + ticks as f64 * DT)).abs() > 1e-8 {
        return Err(format!("wrong persisted epoch: {final_epoch}").into());
    }
    if (persisted.position().raw() - last_position).magnitude() > 1e-6 {
        return Err("persisted position differs from final tick report".into());
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{}: ticks={} setup_ms={:.2} first_ms={:.2} median_ms={:.2} p95_ms={:.2} p99_ms={:.2} max_ms={:.2} aero_ticks={} last_aero_tick={} last_aero_altitude_m={:?} contact_ticks={} final_speed_m_s={:.2} final_epoch_s={:.3}",
        scenario.name(),
        ticks,
        setup_ms,
        first_ms,
        percentile(&times, 0.5),
        percentile(&times, 0.95),
        percentile(&times, 0.99),
        percentile(&times, 1.0),
        aerodynamic_ticks,
        last_aero_tick,
        last_aero_altitude,
        contact_ticks,
        final_speed,
        final_epoch,
    );
    if sql_count {
        let first = sql_after_first.since(sql_before);
        let steady = sql_after_all.since(sql_after_first);
        println!(
            "  sql: first_reads={} first_writes={} steady_reads={} steady_writes={} steady_others={} steady_ticks={}",
            first.reads,
            first.writes,
            steady.reads,
            steady.writes,
            steady.others,
            ticks.saturating_sub(1)
        );
        for (table, count) in sql_trace::top_tables(12) {
            println!("  sql_table {table}: statements={count}");
        }
    }
    if profiling {
        for (name, mut samples) in stage_times {
            samples.sort_by(f64::total_cmp);
            println!(
                "  stage {name}: median_ms={:.2} p95_ms={:.2}",
                percentile(&samples, 0.5),
                percentile(&samples, 0.95)
            );
        }
    }
    tick_session.close().await;
    pool.close().await;
    Ok(())
}

fn percentile(sorted: &[f64], quantile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * quantile).ceil() as usize;
    sorted[index]
}

fn validate(
    report: &rocketcon_app::aeroespacial::VehicleTickReport,
    expected_epoch: f64,
) -> Result<(), Box<dyn Error>> {
    let state = report.physical_state();
    if (state.captured_at_epoch().value() - expected_epoch).abs() > 1e-8
        || !state.position().raw().magnitude().is_finite()
        || !state.velocity().raw().magnitude().is_finite()
        || !report.total_g_load().is_finite()
    {
        return Err(format!("invalid tick state at {expected_epoch}").into());
    }
    Ok(())
}

async fn create_fixture(
    pool: &SqlitePool,
    scenario: Scenario,
) -> Result<(Uuid, Duration), Box<dyn Error>> {
    let vehicle_id = Uuid::parse_str(VEHICLE_ID)?;
    let planet_id = Uuid::parse_str(scenario.planet_id())?;
    let epoch = rocketcon_app::universe::resolve_universe_epoch(pool).await?;
    let environment = rocketcon_app::environment::load_environment_snapshot(
        pool,
        planet_id,
        epoch,
        Duration::new(INITIAL_AT_EPOCH),
    )
    .await?;
    let radius = environment
        .planet
        .equatorial_radius()
        .ok_or("planet has no radius")?
        .value();
    let altitude = match scenario {
        Scenario::Coast => 200_000.0,
        _ => 10_000.0,
    };
    let x = environment.planet_position.raw().0 + radius + altitude;
    let y = environment.planet_position.raw().1;
    let z = environment.planet_position.raw().2;
    let orbital_radius = radius + altitude;
    let mu = 6.67430e-11 * environment.planet.mass().value();
    let relative_speed = match scenario {
        Scenario::Coast => (mu / orbital_radius).sqrt(),
        _ => 100.0,
    };
    let (_, planet_velocity) = rocketcon_app::orbital::soi::resolve_body_state_at_epoch(
        pool,
        planet_id,
        environment.system_id,
        epoch + Duration::new(INITIAL_AT_EPOCH),
    )
    .await?;
    let velocity = planet_velocity.raw();

    sqlx::query(
        "INSERT INTO vehicles (id, name, vehicle_kind) VALUES (?, 'Benchmark Probe', 'Probe')",
    )
    .bind(VEHICLE_ID)
    .execute(pool)
    .await?;
    sqlx::query("INSERT INTO components (id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w) VALUES (?, 'Benchmark CPU', 'Cpu', 100.0, 1.0, 1.0, 5.0)")
        .bind(COMPONENT_ID).execute(pool).await?;
    for index in 0..8 {
        let id = Uuid::from_u128(0xe2897c6a7d044ebc882c87984a740000 + index);
        sqlx::query("INSERT INTO vehicle_components (id, vehicle_id, component_id, stage_index, mount_offset_z_m) VALUES (?, ?, ?, 0, ?)")
            .bind(id.to_string()).bind(VEHICLE_ID).bind(COMPONENT_ID)
            .bind(index as f64).execute(pool).await?;
    }
    sqlx::query("INSERT INTO components (id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w) VALUES (?, 'Benchmark Battery', 'Battery', 80.0, 1.0, 1.0, 0.0)")
        .bind(BATTERY_ID).execute(pool).await?;
    for (key, value) in [
        ("capacity_j", 100_000.0),
        ("max_discharge_power_w", 10_000.0),
        ("max_charge_power_w", 2_000.0),
    ] {
        sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, numeric_value) VALUES (?, ?, ?)")
            .bind(BATTERY_ID).bind(key).bind(value).execute(pool).await?;
    }
    let battery_entry = Uuid::from_u128(0xe2897c6a7d044ebc882c87984a740100);
    sqlx::query("INSERT INTO vehicle_components (id, vehicle_id, component_id, stage_index) VALUES (?, ?, ?, 0)")
        .bind(battery_entry.to_string()).bind(VEHICLE_ID).bind(BATTERY_ID)
        .execute(pool).await?;
    sqlx::query("INSERT INTO energy_reservoir_states (vehicle_component_id, stored_energy_j, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, 90000, ?, ?)")
        .bind(battery_entry.to_string()).bind(epoch.value()).bind(INITIAL_AT_EPOCH)
        .execute(pool).await?;

    sqlx::query("INSERT INTO components (id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w) VALUES (?, 'Benchmark Solar Panel', 'SolarPanel', 20.0, 1.0, 1.0, 0.0)")
        .bind(SOLAR_ID).execute(pool).await?;
    for (key, value) in [
        ("surface_area_m2", 2.0),
        ("conversion_efficiency", 0.2),
        ("max_power_output_w", 500.0),
        ("is_sun_tracking", 1.0),
    ] {
        sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, numeric_value) VALUES (?, ?, ?)")
            .bind(SOLAR_ID).bind(key).bind(value).execute(pool).await?;
    }
    sqlx::query("INSERT INTO vehicle_components (id, vehicle_id, component_id, stage_index) VALUES (?, ?, ?, 0)")
        .bind(Uuid::from_u128(0xe2897c6a7d044ebc882c87984a740101).to_string())
        .bind(VEHICLE_ID).bind(SOLAR_ID).execute(pool).await?;

    if matches!(scenario, Scenario::Powered) {
        sqlx::query("INSERT INTO propellants (id, name, propellant_kind, density_kg_per_m3, is_cryogenic, is_hypergolic) VALUES (?, 'Benchmark Fuel', 'LiquidFuel', 800, 0, 0)")
            .bind(PROPELLANT_ID).execute(pool).await?;
        sqlx::query("INSERT INTO components (id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w) VALUES (?, 'Benchmark Engine', 'Engine', 250.0, 2.0, 1.0, 25.0)")
            .bind(ENGINE_ID).execute(pool).await?;
        for (key, value) in [
            ("specific_impulse_vacuum_s", 300.0),
            ("specific_impulse_sea_level_s", 270.0),
            ("max_thrust_n", 50_000.0),
            ("integral_propellant_mass_kg", 1_000.0),
        ] {
            sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, numeric_value) VALUES (?, ?, ?)")
                .bind(ENGINE_ID).bind(key).bind(value).execute(pool).await?;
        }
        for (key, value) in [
            ("fuel_propellant_id", PROPELLANT_ID),
            ("ignition_type", "SingleBurn"),
        ] {
            sqlx::query("INSERT INTO component_attributes (component_id, attribute_key, text_value) VALUES (?, ?, ?)")
                .bind(ENGINE_ID).bind(key).bind(value).execute(pool).await?;
        }
        let engine_entry = Uuid::from_u128(0xe2897c6a7d044ebc882c87984a740102);
        sqlx::query("INSERT INTO vehicle_components (id, vehicle_id, component_id, stage_index) VALUES (?, ?, ?, 0)")
            .bind(engine_entry.to_string()).bind(VEHICLE_ID).bind(ENGINE_ID)
            .execute(pool).await?;
        sqlx::query("INSERT INTO component_operational_states (vehicle_component_id, load_fraction, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, 0.5, ?, ?)")
            .bind(engine_entry.to_string()).bind(epoch.value()).bind(INITIAL_AT_EPOCH)
            .execute(pool).await?;
    }
    sqlx::query("INSERT INTO vehicle_physical_states (vehicle_id, position_x_m, position_y_m, position_z_m, velocity_x_m_s, velocity_y_m_s, velocity_z_m_s, orientation_q_w, orientation_q_x, orientation_q_y, orientation_q_z, angular_velocity_x_rad_s, angular_velocity_y_rad_s, angular_velocity_z_rad_s, reference_body_id, captured_universe_epoch_s, captured_at_epoch_s) VALUES (?, ?, ?, ?, ?, ?, ?, 1, 0, 0, 0, 0, 0, 0, ?, ?, ?)")
        .bind(VEHICLE_ID).bind(x).bind(y).bind(z)
        .bind(velocity.0).bind(velocity.1 + relative_speed).bind(velocity.2)
        .bind(scenario.planet_id()).bind(epoch.value()).bind(INITIAL_AT_EPOCH)
        .execute(pool).await?;
    if matches!(scenario, Scenario::Coast) {
        sqlx::query("INSERT INTO vehicle_trajectory_patches (id, vehicle_id, reference_body_id, start_universe_epoch_s, gravitational_parameter_m3_s2, patch_type, semi_major_axis_m, eccentricity, inclination_rad, longitude_of_ascending_node_rad, argument_of_periapsis_rad, true_anomaly_at_epoch_rad) VALUES (?, ?, ?, ?, ?, 'conic', ?, 0, 0, 0, 0, 0)")
            .bind(Uuid::from_u128(0xe2897c6a7d044ebc882c87984a749999).to_string())
            .bind(VEHICLE_ID).bind(scenario.planet_id())
            .bind(epoch.value() + INITIAL_AT_EPOCH).bind(mu)
            .bind(orbital_radius).execute(pool).await?;
    }
    Ok((vehicle_id, epoch))
}
