use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use rocketcon_app::orbital::hybrid_regime::{RegimePolicy, choose_regime};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_db::repositories::vehicle_physical_state;
use rocketcon_sim::ForecastReader;
use sqlx::SqlitePool;
use std::error::Error;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const G: f64 = 6.67430e-11;
const AU_M: f64 = 149_597_870_700.0;
const VEHICLE: &str = "715c3661-e69f-475c-8c78-64927ae12b01";
const SYSTEM: u128 = 0xc1500000000000000000000000000001;
const SUN: u128 = 0xc1500000000000000000000000000002;
const JUPITER: u128 = 0xc1500000000000000000000000000003;
const IO: u128 = 0xc1500000000000000000000000000004;
const EUROPA: u128 = 0xc1500000000000000000000000000005;
const GANYMEDE: u128 = 0xc1500000000000000000000000000006;
const CALLISTO: u128 = 0xc1500000000000000000000000000007;

struct Satellite {
    id: u128,
    name: &'static str,
    kind: &'static str,
    gm_km3_s2: f64,
    radius_km: f64,
    a_km: f64,
    eccentricity: f64,
    inclination_deg: f64,
    node_deg: f64,
    peri_deg: f64,
    mean_deg: f64,
}

const MOONS: [Satellite; 4] = [
    Satellite {
        id: IO,
        name: "Io",
        kind: "Telluric",
        gm_km3_s2: 5959.91547,
        radius_km: 1821.49,
        a_km: 421800.0,
        eccentricity: 0.004,
        inclination_deg: 0.0,
        node_deg: 0.0,
        peri_deg: 49.1,
        mean_deg: 330.9,
    },
    Satellite {
        id: EUROPA,
        name: "Europa",
        kind: "IcyBody",
        gm_km3_s2: 3202.71210,
        radius_km: 1560.80,
        a_km: 671100.0,
        eccentricity: 0.009,
        inclination_deg: 0.5,
        node_deg: 184.0,
        peri_deg: 45.0,
        mean_deg: 345.4,
    },
    Satellite {
        id: GANYMEDE,
        name: "Ganymede",
        kind: "IcyBody",
        gm_km3_s2: 9887.83275,
        radius_km: 2631.20,
        a_km: 1070400.0,
        eccentricity: 0.001,
        inclination_deg: 0.2,
        node_deg: 58.5,
        peri_deg: 198.3,
        mean_deg: 324.8,
    },
    Satellite {
        id: CALLISTO,
        name: "Callisto",
        kind: "IcyBody",
        gm_km3_s2: 7179.28340,
        radius_km: 2410.30,
        a_km: 1882700.0,
        eccentricity: 0.007,
        inclination_deg: 0.3,
        node_deg: 309.1,
        peri_deg: 43.8,
        mean_deg: 87.4,
    },
];

fn uuid(value: u128) -> String {
    Uuid::from_u128(value).to_string()
}

fn rad(deg: f64) -> f64 {
    deg.to_radians()
}

async fn insert_star(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO star_systems (id, name) VALUES (?, ?)")
        .bind(uuid(SYSTEM))
        .bind("C15 JPL Jovian Validation")
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO stars (id, star_system_id, name, kind, mass_kg, radius_m, effective_temperature_k) VALUES (?, ?, 'Sol C15', 'Star', ?, ?, ?)")
        .bind(uuid(SUN)).bind(uuid(SYSTEM))
        .bind(1.32712440041279419e20 / G).bind(695_700_000.0)
        .bind(5772.0).execute(pool).await?;
    Ok(())
}

async fn insert_jupiter(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO planets (id, star_system_id, parent_star_id, name, kind, mass_kg, equatorial_radius_m, polar_radius_m, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad) VALUES (?, ?, ?, 'Jupiter C15', 'GasGiant', ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(uuid(JUPITER)).bind(uuid(SYSTEM)).bind(uuid(SUN))
        .bind(126_686_531.9e9 / G).bind(71_492_000.0).bind(66_854_000.0)
        .bind(5.20336301 * AU_M).bind(0.04839266)
        .bind(rad(1.30530)).bind(rad(100.55615))
        .bind(rad(14.75385 - 100.55615)).bind(rad(34.40438 - 14.75385))
        .execute(pool).await?;
    Ok(())
}

async fn insert_moons(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for moon in &MOONS {
        sqlx::query("INSERT INTO planets (id, star_system_id, parent_planet_id, name, kind, mass_kg, equatorial_radius_m, polar_radius_m, semi_major_axis_m, eccentricity, inclination_rad, longitude_ascending_node_rad, argument_periapsis_rad, mean_anomaly_at_epoch_rad) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(uuid(moon.id)).bind(uuid(SYSTEM)).bind(uuid(JUPITER))
            .bind(moon.name).bind(moon.kind).bind(moon.gm_km3_s2 * 1e9 / G)
            .bind(moon.radius_km * 1000.0).bind(moon.radius_km * 1000.0)
            .bind(moon.a_km * 1000.0).bind(moon.eccentricity)
            .bind(rad(moon.inclination_deg)).bind(rad(moon.node_deg))
            .bind(rad(moon.peri_deg)).bind(rad(moon.mean_deg))
            .execute(pool).await?;
    }
    Ok(())
}

async fn set_vehicle(pool: &SqlitePool, mode: &str) -> Result<(), Box<dyn Error>> {
    let id = Uuid::parse_str(VEHICLE)?;
    let initial = vehicle_physical_state::get_by_vehicle_id(pool, &id)
        .await?
        .ok_or("fixture source has no vehicle")?;
    let catalog = EphemerisCatalog::load(pool, Uuid::from_u128(SYSTEM)).await?;
    let states = catalog.states(Duration::new(1.0))?;
    let jupiter = states.get(&Uuid::from_u128(JUPITER)).ok_or("no Jupiter")?;
    let (position, velocity, reference) = if mode == "jupiter-coast" {
        let rel = Vector3::new(300_000_000.0, 0.0, 0.0);
        let orbital = (126_686_531.9e9 / rel.magnitude()).sqrt();
        (
            jupiter.position.raw() + rel,
            jupiter.velocity.raw() + Vector3::new(0.0, orbital, 0.0),
            Uuid::from_u128(JUPITER),
        )
    } else {
        let moon_id = match mode {
            "io-prograde" | "io-retrograde" => IO,
            "europa-flyby" => EUROPA,
            "ganymede-flyby" => GANYMEDE,
            _ => return Err("unknown Jovian scenario".into()),
        };
        let moon = states.get(&Uuid::from_u128(moon_id)).ok_or("no moon")?;
        let outward = moon.position.raw() - jupiter.position.raw();
        let radial = outward / outward.magnitude();
        let tangent = Vector3::new(-radial.1, radial.0, 0.0);
        let direction = if mode == "io-retrograde" { -1.0 } else { 1.0 };
        let position =
            moon.position.raw() - tangent * (direction * 18_000_000.0) + radial * 8_000_000.0;
        let velocity = moon.velocity.raw() + tangent * (direction * 3_500.0);
        (position, velocity, Uuid::from_u128(JUPITER))
    };
    let new = VehiclePhysicalState::new_with_max_q(
        id,
        Position::from_raw(position),
        VelocityVector::from_raw(velocity),
        initial.orientation(),
        initial.angular_velocity(),
        reference,
        Duration::new(1.0),
        Duration::new(1.0),
        initial.max_dynamic_pressure(),
        initial.max_dynamic_pressure_epoch(),
    )?;
    vehicle_physical_state::upsert(pool, &new).await?;
    sqlx::query("UPDATE vehicles SET name = ? WHERE id = ?")
        .bind(format!("C15 JPL {mode}"))
        .bind(VEHICLE)
        .execute(pool)
        .await?;
    Ok(())
}

async fn build_one(source: &Path, destination: &Path, mode: &str) -> Result<(), Box<dyn Error>> {
    if destination.exists() {
        return Err(format!("refusing to overwrite {}", destination.display()).into());
    }
    std::fs::copy(source, destination)?;
    let pool = astronomicon_db::connection::open_pool_path(destination).await?;
    let result: Result<(), Box<dyn Error>> = async {
        sqlx::query("UPDATE universe_state SET seconds_since_j2000_epoch = 1.0")
            .execute(&pool)
            .await?;
        insert_star(&pool).await?;
        insert_jupiter(&pool).await?;
        insert_moons(&pool).await?;
        set_vehicle(&pool, mode).await?;
        sqlx::query("DELETE FROM vehicle_trajectory_patches WHERE vehicle_id = ?")
            .bind(VEHICLE)
            .execute(&pool)
            .await?;
        let current = vehicle_physical_state::get_by_vehicle_id(&pool, &Uuid::parse_str(VEHICLE)?)
            .await?
            .ok_or("saved C15 probe vehicle missing")?;
        let ephemeris = EphemerisCatalog::load(&pool, Uuid::from_u128(SYSTEM)).await?;
        let regime = choose_regime(
            &ephemeris,
            &current,
            Duration::new(0.02),
            false,
            RegimePolicy::flight(),
        )?;
        println!("FIXTURE_SELECTOR mode={mode} regime={regime:?}");
        let reader = ForecastReader::open(destination, Uuid::parse_str(VEHICLE)?).await?;
        let preview = reader.preview(false).await?;
        println!(
            "FIXTURE={} sources=6 arcs={} forecast_s={:.3} completed_s={:.3}",
            destination.display(),
            preview.future_arcs.len(),
            preview.forecast_target_seconds,
            preview.forecast_completed_seconds
        );
        reader.close().await;
        Ok(())
    }
    .await;
    pool.close().await;
    if result.is_err() {
        std::fs::remove_file(destination)?;
    }
    result
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "saves/c15-multiarc-inbound.db".into()),
    );
    let directory = PathBuf::from(std::env::args().nth(2).unwrap_or_else(|| "saves".into()));
    if !source.is_file() {
        return Err("C15 source vehicle save unavailable".into());
    }
    std::fs::create_dir_all(&directory)?;
    let only = std::env::args().nth(3);
    for mode in [
        "jupiter-coast",
        "io-prograde",
        "io-retrograde",
        "europa-flyby",
        "ganymede-flyby",
    ] {
        if only.as_deref().is_some_and(|selected| selected != mode) {
            continue;
        }
        let path = directory.join(format!("c15-jpl-{mode}.db"));
        build_one(&source, &path, mode).await?;
    }
    Ok(())
}
