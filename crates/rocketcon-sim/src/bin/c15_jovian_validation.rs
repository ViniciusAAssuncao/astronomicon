use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::Duration;
use rocketcon_app::orbital::hybrid_regime::{RegimePolicy, choose_regime};
use rocketcon_core::math::orbital::n_body::dop853::Dop853Config;
use rocketcon_db::repositories::vehicle_physical_state;
use rocketcon_sim::{
    ForecastReader, RocketconSession, catalog_nbody::integrate_catalog_trajectory,
};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Instant;
use uuid::Uuid;

const VEHICLE: &str = "715c3661-e69f-475c-8c78-64927ae12b01";
const SYSTEM: u128 = 0xc1500000000000000000000000000001;
const JUPITER: u128 = 0xc1500000000000000000000000000003;
const IO: u128 = 0xc1500000000000000000000000000004;
const EUROPA: u128 = 0xc1500000000000000000000000000005;
const GANYMEDE: u128 = 0xc1500000000000000000000000000006;

fn config() -> Dop853Config<6> {
    Dop853Config {
        absolute_tolerances: [0.01, 0.01, 0.01, 1e-5, 1e-5, 1e-5],
        relative_tolerance: 1e-9,
        initial_step_s: 30.0,
        minimum_step_s: 1e-8,
        maximum_step_s: 300.0,
        maximum_attempts: 1024,
        maximum_evaluations: 16000,
        maximum_segments: 2048,
    }
}

fn energy(state: [f64; 6], center_pos: [f64; 3], center_vel: [f64; 3]) -> f64 {
    let r = (0..3)
        .map(|i| (state[i] - center_pos[i]).powi(2))
        .sum::<f64>()
        .sqrt();
    let v2 = (0..3)
        .map(|i| (state[i + 3] - center_vel[i]).powi(2))
        .sum::<f64>();
    0.5 * v2 - 126_686_531.9e9 / r
}

fn array(position: astronomicon_core::units::Vector3) -> [f64; 3] {
    [position.0, position.1, position.2]
}

async fn validate_one(file: &Path, work: &Path) -> Result<(), Box<dyn Error>> {
    let name = file
        .file_stem()
        .ok_or("invalid save name")?
        .to_string_lossy();
    let id = Uuid::parse_str(VEHICLE)?;
    let read = astronomicon_db::connection::open_read_only_pool_path(file).await?;
    let state = vehicle_physical_state::get_by_vehicle_id(&read, &id)
        .await?
        .ok_or("no vehicle")?;
    let epoch = state.captured_total_epoch();
    let catalog = EphemerisCatalog::load(&read, Uuid::from_u128(SYSTEM)).await?;
    let total = catalog.gravitating_bodies().len();
    if total != 6 {
        return Err(format!("expected 6 real gravitating sources, got {total}").into());
    }
    let regime = choose_regime(
        &catalog,
        &state,
        Duration::new(0.02),
        false,
        RegimePolicy::flight(),
    )?;
    println!("REGIME_VALIDATION save={name} regime={regime:?}");
    let span = Duration::new(10_800.0);
    let start = Instant::now();
    let integrated = integrate_catalog_trajectory(
        &catalog,
        epoch,
        state.position(),
        state.velocity(),
        span,
        None,
        config(),
        || false,
    )
    .map_err(|error| format!("Jovian integration failure: {error:?}"))?;
    let full_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    let only_jupiter = integrate_catalog_trajectory(
        &catalog,
        epoch,
        state.position(),
        state.velocity(),
        span,
        Some(&[Uuid::from_u128(JUPITER)]),
        config(),
        || false,
    )
    .map_err(|error| format!("Jovian integration failure: {error:?}"))?;
    let primary_ms = start.elapsed().as_secs_f64() * 1000.0;
    let end_epoch = epoch + span;
    let end_states = catalog.states(end_epoch)?;
    let jupiter = end_states
        .get(&Uuid::from_u128(JUPITER))
        .ok_or("Jupiter missing at the end")?;
    let central_position = array(jupiter.position.raw());
    let central_velocity = array(jupiter.velocity.raw());
    let start_states = catalog.states(epoch)?;
    let start_jupiter = start_states
        .get(&Uuid::from_u128(JUPITER))
        .ok_or("Jupiter missing at start")?;
    let start_position = state.position().raw();
    let start_velocity = state.velocity().raw();
    let starting_energy = energy(
        [
            start_position.0,
            start_position.1,
            start_position.2,
            start_velocity.0,
            start_velocity.1,
            start_velocity.2,
        ],
        array(start_jupiter.position.raw()),
        array(start_jupiter.velocity.raw()),
    );
    let multi_energy = energy(
        integrated.integrated.final_state,
        central_position,
        central_velocity,
    );
    let central_energy = energy(
        only_jupiter.integrated.final_state,
        central_position,
        central_velocity,
    );
    let difference = (0..3)
        .map(|axis| {
            (integrated.integrated.final_state[axis] - only_jupiter.integrated.final_state[axis])
                .powi(2)
        })
        .sum::<f64>()
        .sqrt();
    let target = if name.contains("io-") {
        Some(IO)
    } else if name.contains("europa-") {
        Some(EUROPA)
    } else if name.contains("ganymede-") {
        Some(GANYMEDE)
    } else {
        None
    };
    let mut closest = f64::INFINITY;
    if let Some(body) = target {
        let id = Uuid::from_u128(body);
        for segment in &integrated.integrated.segments {
            for time in [
                segment.start_seconds,
                (segment.start_seconds + segment.valid_until_seconds) * 0.5,
                segment.valid_until_seconds,
            ] {
                let Some(position) = segment.evaluate(time) else {
                    continue;
                };
                let center = catalog.positions_for_ids_at_offset(epoch, time, &[id])?;
                let offset = center.get(&id).ok_or("moon missing")?.raw();
                let d = ((position[0] - offset.0).powi(2)
                    + (position[1] - offset.1).powi(2)
                    + (position[2] - offset.2).powi(2))
                .sqrt();
                closest = closest.min(d);
            }
        }
        let radius = catalog.radius_of_body(id).ok_or("moon radius missing")?;
        if closest <= radius {
            return Err("forecast trajectory intersected a moon".into());
        }
        let others = catalog
            .gravitating_bodies()
            .into_iter()
            .map(|(body_id, _)| body_id)
            .filter(|body_id| *body_id != id)
            .collect::<Vec<_>>();
        let without_target = integrate_catalog_trajectory(
            &catalog,
            epoch,
            state.position(),
            state.velocity(),
            span,
            Some(&others),
            config(),
            || false,
        )
        .map_err(|error| format!("exclude moon control failed: {error:?}"))?;
        let baseline_energy = energy(
            without_target.integrated.final_state,
            central_position,
            central_velocity,
        );
        let contribution = multi_energy - baseline_energy;
        let moon_shift = (0..3)
            .map(|axis| {
                (integrated.integrated.final_state[axis]
                    - without_target.integrated.final_state[axis])
                    .powi(2)
            })
            .sum::<f64>()
            .sqrt();
        println!(
            "MOON_ISOLATION save={name} moon={id} delta_energy_m2s2={contribution:.3} position_effect_m={moon_shift:.3}"
        );
    }
    println!(
        "ORBITAL_VALIDATION save={name} sources={total} span_s={} nbody_ms={full_ms:.1} single_ms={primary_ms:.1} separation_m={difference:.3} energy_multi_minus_single_m2s2={:.3} closest_target_km={:.2} impact={}",
        span.value(),
        multi_energy - central_energy,
        closest / 1000.0,
        integrated.impact.is_some()
    );

    println!(
        "JOVIAN_ENERGY save={name} delta_nbody_m2s2={:.3} delta_primary_m2s2={:.3}",
        multi_energy - starting_energy,
        central_energy - starting_energy
    );
    let reader = ForecastReader::open(file, id).await?;
    let preview = reader.preview(false).await?;
    for arcs in preview.future_arcs.windows(2) {
        let first = arcs[0].points_m.last().ok_or("empty forecast arc")?;
        let second = arcs[1].points_m.first().ok_or("empty succeeding arc")?;
        let gap = first
            .iter()
            .zip(second.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        if !gap.is_finite() || gap > 1.0 {
            return Err(format!("forecast arc continuity gap: {gap} m").into());
        }
    }
    println!(
        "MISSION_FORECAST save={name} arcs={} resolved_s={:.3} target_s={:.3} complete={} events={}",
        preview.future_arcs.len(),
        preview.forecast_completed_seconds,
        preview.forecast_target_seconds,
        preview.forecast_complete,
        preview
            .forecast_events
            .iter()
            .map(|event| format!("{}:{}@{:.3}s", event.kind, event.body_name, event.seconds))
            .collect::<Vec<_>>()
            .join(";")
    );
    reader.close().await;
    let copy = work.join(format!("{name}-ticks.db"));
    if copy.exists() {
        return Err(format!("refusing to overwrite tick test copy {}", copy.display()).into());
    }
    std::fs::copy(file, &copy)?;
    let mut session = RocketconSession::load(&copy, id).await?;
    let old_epoch = session.snapshot().total_epoch_seconds;
    let start = Instant::now();
    for _ in 0..3 {
        let next = session.step(0.02).await?;
        if next.total_epoch_seconds <= old_epoch {
            return Err("tick failed to advance".into());
        }
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "TICK_VALIDATION save={name} ticks=3 total_ms={elapsed_ms:.2} persisted_epoch_s={:.6}",
        session.snapshot().total_epoch_seconds
    );
    session.close().await;
    read.close().await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let saves = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "saves".into()));
    let work = PathBuf::from(
        std::env::args()
            .nth(2)
            .unwrap_or_else(|| "target/c15-jpl-validation".into()),
    );
    std::fs::create_dir_all(&work)?;
    for name in [
        "jupiter-coast",
        "io-prograde",
        "io-retrograde",
        "europa-flyby",
        "ganymede-flyby",
        "io-europa-transfer",
    ] {
        validate_one(&saves.join(format!("c15-jpl-{name}.db")), &work).await?;
    }
    Ok(())
}
