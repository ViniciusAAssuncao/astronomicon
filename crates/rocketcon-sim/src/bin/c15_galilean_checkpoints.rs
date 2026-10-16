use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::n_body::dop853::Dop853Config;
use rocketcon_db::repositories::vehicle_physical_state;
use rocketcon_sim::{ForecastReader, catalog_nbody::integrate_catalog_trajectory};
use std::error::Error;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const VEHICLE: &str = "715c3661-e69f-475c-8c78-64927ae12b01";
const SYSTEM: u128 = 0xc1500000000000000000000000000001;
const JUPITER: u128 = 0xc1500000000000000000000000000003;
const EUROPA: u128 = 0xc1500000000000000000000000000005;

fn config(duration: f64) -> Dop853Config<6> {
    Dop853Config {
        absolute_tolerances: [0.01, 0.01, 0.01, 1e-5, 1e-5, 1e-5],
        relative_tolerance: 1e-9,
        initial_step_s: 50.0,
        minimum_step_s: 1e-8,
        maximum_step_s: (duration / 100.0).min(900.0),
        maximum_attempts: 1600,
        maximum_evaluations: 24000,
        maximum_segments: 1200,
    }
}

async fn create_checkpoint(
    source: &Path,
    dest: &Path,
    label: &str,
    elapsed: f64,
    reference: Uuid,
) -> Result<(), Box<dyn Error>> {
    if dest.exists() {
        return Err(format!(
            "checkpoint exists; refusing to overwrite {}",
            dest.display()
        )
        .into());
    }
    let read = astronomicon_db::connection::open_read_only_pool_path(source).await?;
    let vehicle_id = Uuid::parse_str(VEHICLE)?;
    let initial = vehicle_physical_state::get_by_vehicle_id(&read, &vehicle_id)
        .await?
        .ok_or("initial probe state missing")?;
    let epoch = initial.captured_total_epoch();
    let catalog = EphemerisCatalog::load(&read, Uuid::from_u128(SYSTEM)).await?;
    let solved = integrate_catalog_trajectory(
        &catalog,
        epoch,
        initial.position(),
        initial.velocity(),
        Duration::new(elapsed),
        None,
        config(elapsed),
        || false,
    )
    .map_err(|error| format!("checkpoint trajectory failed: {error:?}"))?;
    if solved.impact.is_some() {
        return Err("checkpoint path collided with celestial surface".into());
    }
    let state = solved.integrated.final_state;
    let current_position = Position::from_raw(Vector3::new(state[0], state[1], state[2]));
    let current_velocity = VelocityVector::from_raw(Vector3::new(state[3], state[4], state[5]));
    let europa_id = Uuid::from_u128(EUROPA);
    let centers = catalog.positions_for_ids_at_offset(epoch, elapsed, &[europa_id])?;
    let moon = centers.get(&europa_id).ok_or("Europa position missing")?;
    let separation = (current_position.raw() - moon.raw()).magnitude();
    let radius = catalog
        .radius_of_body(europa_id)
        .ok_or("Europa radius missing")?;
    if separation <= radius {
        return Err("checkpoint inside Europa solid radius".into());
    }
    read.close().await;
    std::fs::copy(source, dest)?;
    let pool = astronomicon_db::connection::open_pool_path(dest).await?;
    let result: Result<(), Box<dyn Error>> = async {
        let next_universe_epoch = initial.captured_universe_epoch() + Duration::new(elapsed);
        let saved = VehiclePhysicalState::new_with_max_q(
            vehicle_id,
            current_position,
            current_velocity,
            initial.orientation(),
            initial.angular_velocity(),
            reference,
            next_universe_epoch,
            initial.captured_at_epoch(),
            initial.max_dynamic_pressure(),
            initial.max_dynamic_pressure_epoch(),
        )?;
        vehicle_physical_state::upsert(&pool, &saved).await?;
        sqlx::query("UPDATE universe_state SET seconds_since_j2000_epoch = ?")
            .bind(next_universe_epoch.value())
            .execute(&pool)
            .await?;
        sqlx::query("UPDATE vehicles SET name = ? WHERE id = ?")
            .bind(format!("C15 JPL Europa {label}"))
            .bind(VEHICLE)
            .execute(&pool)
            .await?;
        sqlx::query("DELETE FROM vehicle_trajectory_patches WHERE vehicle_id = ?")
            .bind(VEHICLE)
            .execute(&pool)
            .await?;
        let (busy, _, _): (i64, i64, i64) = sqlx::query_as("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_one(&pool)
            .await?;
        if busy != 0 {
            return Err("checkpoint SQLite WAL busy".into());
        }
        Ok(())
    }
    .await;
    pool.close().await;
    if result.is_err() {
        std::fs::remove_file(dest)?;
    }
    result?;
    println!(
        "MISSION_CHECKPOINT={} elapsed_s={elapsed:.6} moon_distance_km={:.3} altitude_km={:.3} reference={reference}",
        dest.display(),
        separation / 1000.0,
        (separation - radius) / 1000.0
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "saves/c15-jpl-io-europa-transfer.db".into()),
    );
    let directory = PathBuf::from(std::env::args().nth(2).unwrap_or_else(|| "saves".into()));
    if !source.is_file() {
        return Err("Io-Europa transfer source does not exist".into());
    }
    std::fs::create_dir_all(&directory)?;
    let reader = ForecastReader::open(&source, Uuid::parse_str(VEHICLE)?).await?;
    let preview = reader.preview(false).await?;
    let entry = preview
        .forecast_events
        .iter()
        .find(|event| event.kind == 0 && event.body_name == "Europa")
        .ok_or("no Europa SOI entry")?
        .seconds;
    let exit = preview
        .forecast_events
        .iter()
        .find(|event| event.kind == 1 && event.body_name == "Europa")
        .ok_or("no Europa SOI exit")?
        .seconds;
    reader.close().await;
    if exit <= entry + 1.0 {
        return Err("invalid Europa entry/exit sequence".into());
    }
    for (name, label, at, reference) in [
        (
            "europa-entry",
            "SOI entry",
            entry - 0.4,
            Uuid::from_u128(JUPITER),
        ),
        (
            "europa-close",
            "close approach",
            (entry + exit) * 0.5,
            Uuid::from_u128(EUROPA),
        ),
        (
            "europa-exit",
            "SOI exit",
            exit - 0.4,
            Uuid::from_u128(EUROPA),
        ),
    ] {
        create_checkpoint(
            &source,
            &directory.join(format!("c15-jpl-{name}.db")),
            label,
            at,
            reference,
        )
        .await?;
    }
    println!("C15_GALILEAN_CHECKPOINTS_PASS");
    Ok(())
}
