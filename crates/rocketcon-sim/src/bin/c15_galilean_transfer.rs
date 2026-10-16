use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::n_body::dop853::Dop853Config;
use rocketcon_db::repositories::vehicle_physical_state;
use rocketcon_sim::{ForecastReader, catalog_nbody::integrate_catalog_trajectory};
use std::error::Error;
use std::f64::consts::PI;
use std::path::Path;
use uuid::Uuid;

const VEHICLE: &str = "715c3661-e69f-475c-8c78-64927ae12b01";
const SYSTEM: u128 = 0xc1500000000000000000000000000001;
const JUPITER: u128 = 0xc1500000000000000000000000000003;
const IO: u128 = 0xc1500000000000000000000000000004;
const EUROPA: u128 = 0xc1500000000000000000000000000005;
const MU_J: f64 = 126_686_531.9e9;

fn config(seconds: f64) -> Dop853Config<6> {
    Dop853Config {
        absolute_tolerances: [0.05, 0.05, 0.05, 1e-5, 1e-5, 1e-5],
        relative_tolerance: 1e-9,
        initial_step_s: 60.0,
        minimum_step_s: 1e-8,
        maximum_step_s: (seconds / 80.0).min(1800.0),
        maximum_attempts: 1600,
        maximum_evaluations: 24000,
        maximum_segments: 1200,
    }
}

fn unit(vector: Vector3) -> Vector3 {
    vector / vector.magnitude()
}

async fn find_departure(
    catalog: &EphemerisCatalog,
) -> Result<(f64, Vector3, Vector3, f64), Box<dyn Error>> {
    let j = Uuid::from_u128(JUPITER);
    let io = Uuid::from_u128(IO);
    let eu = Uuid::from_u128(EUROPA);
    let mut best = (
        f64::INFINITY,
        1.0_f64,
        Vector3::zero(),
        Vector3::zero(),
        0.0,
    );
    for (step, increment) in [(900.0, 0.0), (30.0, 900.0)] {
        let (start, stop) = if increment == 0.0 {
            (1.0, 7.0 * 86_400.0)
        } else {
            ((best.1 - increment).max(1.0), best.1 + increment)
        };
        let mut t0 = start;
        while t0 <= stop {
            let first = catalog.states(Duration::new(t0))?;
            let center = first.get(&j).ok_or("missing Jupiter")?;
            let io_state = first.get(&io).ok_or("missing Io")?;
            let outward = io_state.position.raw() - center.position.raw();
            let radius = outward.magnitude() + 30_000_000.0;
            let time = PI * (((radius + 671_100_000.0) * 0.5).powi(3) / MU_J).sqrt();
            let future = catalog.states(Duration::new(t0 + time))?;
            let europa = future.get(&eu).ok_or("missing Europa")?;
            let jupiter = future.get(&j).ok_or("missing Jupiter at arrival")?;
            let relative = unit(europa.position.raw() - jupiter.position.raw());
            let alignment = (unit(outward) + relative).magnitude();
            if alignment < best.0 {
                best = (alignment, t0, outward, center.velocity.raw(), time);
            }
            t0 += step;
        }
    }
    let direction = unit(best.2);
    let tangent = Vector3::new(-direction.1, direction.0, 0.0);
    let r0 = best.2.magnitude() + 30_000_000.0;
    let transfer_a = (r0 + 671_100_000.0) * 0.5;
    let transfer_speed = (MU_J * (2.0 / r0 - 1.0 / transfer_a)).sqrt();
    Ok((best.1, direction, best.3 + tangent * transfer_speed, best.4))
}

async fn closest_to_europa(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    position: Position,
    velocity: VelocityVector,
    seconds: f64,
) -> Result<(f64, f64), Box<dyn Error>> {
    let integrated = integrate_catalog_trajectory(
        catalog,
        epoch,
        position,
        velocity,
        Duration::new(seconds),
        None,
        config(seconds),
        || false,
    )
    .map_err(|error| format!("Galilean transfer propagation: {error:?}"))?;
    if integrated.impact.is_some() {
        return Err("transfer intersects a celestial surface".into());
    }
    let mut closest = f64::INFINITY;
    let mut when = 0.0;
    let europa_id = Uuid::from_u128(EUROPA);
    for segment in &integrated.integrated.segments {
        for i in 0..=4 {
            let t = segment.start_seconds
                + (segment.valid_until_seconds - segment.start_seconds) * f64::from(i) / 4.0;
            let Some(state) = segment.evaluate(t) else {
                continue;
            };
            let positions = catalog.positions_for_ids_at_offset(epoch, t, &[europa_id])?;
            let center = positions
                .get(&europa_id)
                .ok_or("Europa ephemeris missing")?
                .raw();
            let distance = ((state[0] - center.0).powi(2)
                + (state[1] - center.1).powi(2)
                + (state[2] - center.2).powi(2))
            .sqrt();
            if distance < closest {
                closest = distance;
                when = t;
            }
        }
    }
    Ok((closest, when))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "saves/c15-jpl-jupiter-coast.db".into());
    let output = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "saves/c15-jpl-io-europa-transfer.db".into());
    if Path::new(&output).exists() {
        return Err("refusing to overwrite existing C15 transfer save".into());
    }
    std::fs::copy(&source, &output)?;
    let pool = astronomicon_db::connection::open_pool_path(Path::new(&output)).await?;
    let result: Result<(), Box<dyn Error>> = async {
        let catalog = EphemerisCatalog::load(&pool, Uuid::from_u128(SYSTEM)).await?;
        let (t0, outward, baseline_velocity, nominal_transfer) = find_departure(&catalog).await?;
        let states = catalog.states(Duration::new(t0 + 1.0))?;
        let center = states.get(&Uuid::from_u128(JUPITER)).ok_or("missing Jupiter")?;
        let io_state = states.get(&Uuid::from_u128(IO)).ok_or("missing Io")?;
        let position = Position::from_raw(io_state.position.raw() + outward * 30_000_000.0);
        let tangent = Vector3::new(-outward.1, outward.0, 0.0);
        let mut best = (f64::INFINITY, baseline_velocity, 0.0);
        for correction in [-300.0, -150.0, 0.0, 150.0, 300.0] {
            let velocity = VelocityVector::from_raw(baseline_velocity + tangent * correction);
            let (distance, elapsed) = closest_to_europa(
                &catalog, Duration::new(t0 + 1.0), position, velocity,
                nominal_transfer + 15_000.0,
            ).await?;
            if distance < best.0 {
                best = (distance, velocity.raw(), elapsed);
            }
        }
        println!("GALILEAN_CHAIN epoch_j2000_s={t0:.3} nominal_transfer_s={nominal_transfer:.3} europa_closest_km={:.3} elapsed_s={:.3}",
            best.0 / 1000.0, best.2);
        if best.0 > 75_000_000.0 {
            return Err("Io-Europa transfer missed the validation encounter (<75000km)".into());
        }
        let id = Uuid::parse_str(VEHICLE)?;
        let prior = vehicle_physical_state::get_by_vehicle_id(&pool, &id)
            .await?.ok_or("missing spacecraft")?;
        sqlx::query("UPDATE universe_state SET seconds_since_j2000_epoch = ?")
            .bind(t0).execute(&pool).await?;
        let state = VehiclePhysicalState::new_with_max_q(
            id, position, VelocityVector::from_raw(best.1),
            prior.orientation(), prior.angular_velocity(), Uuid::from_u128(JUPITER),
            Duration::new(t0), Duration::new(1.0),
            prior.max_dynamic_pressure(), prior.max_dynamic_pressure_epoch(),
        )?;
        vehicle_physical_state::upsert(&pool, &state).await?;
        sqlx::query("UPDATE vehicles SET name = 'C15 JPL Io Europa Transfer' WHERE id = ?")
            .bind(VEHICLE).execute(&pool).await?;
        sqlx::query("DELETE FROM vehicle_trajectory_patches WHERE vehicle_id = ?")
            .bind(VEHICLE).execute(&pool).await?;
        let reader = ForecastReader::open(&output, id).await?;
        let preview = reader.preview(false).await?;
        println!("CHAIN_SAVE={} arcs={} forecast_target_s={:.3} reached_s={:.3} complete={}",
            output, preview.future_arcs.len(), preview.forecast_target_seconds,
            preview.forecast_completed_seconds, preview.forecast_complete);
        reader.close().await;
        let _ = center;
        Ok(())
    }.await;
    pool.close().await;
    if result.is_err() {
        std::fs::remove_file(&output)?;
    }
    result
}
