use rocketcon_sim::{FlightEventKind, RocketconSession};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Instant;
use uuid::Uuid;

const VEHICLE: &str = "715c3661-e69f-475c-8c78-64927ae12b01";
const JUPITER: u128 = 0xc1500000000000000000000000000003;
const EUROPA: u128 = 0xc1500000000000000000000000000005;

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

async fn verify_one(
    source: &Path,
    dir: &Path,
    label: &str,
    expected_body: Uuid,
    expected_event: Option<FlightEventKind>,
) -> Result<(), Box<dyn Error>> {
    let dest = dir.join(format!("c15-jpl-{label}-soi-ticks.db"));
    if dest.exists() {
        return Err(format!("refusing to overwrite {}", dest.display()).into());
    }
    std::fs::copy(source, &dest)?;
    let id = Uuid::parse_str(VEHICLE)?;
    let mut session = RocketconSession::load(&dest, id).await?;
    let start_epoch = session.snapshot().total_epoch_seconds;
    let initial_ref = session.snapshot().reference_body_id;
    let start = Instant::now();
    let mut max_displacement_residual_m = 0.0_f64;
    let mut largest_tick_ms = 0.0_f64;
    for _ in 0..36 {
        let previous = session.snapshot().clone();
        let tick_start = Instant::now();
        let current = session.step(0.02).await?;
        largest_tick_ms = largest_tick_ms.max(tick_start.elapsed().as_secs_f64() * 1e3);
        let average_velocity: [f64; 3] = std::array::from_fn(|axis| {
            (previous.velocity_m_s[axis] + current.velocity_m_s[axis]) * 0.5 * 0.02
        });
        let expected_position =
            std::array::from_fn(|axis| previous.position_m[axis] + average_velocity[axis]);
        let residual = distance(current.position_m, expected_position);
        if !residual.is_finite() || residual > 10.0 {
            return Err(
                format!("inertial trajectory discontinuity at {label}: {residual} m").into(),
            );
        }
        max_displacement_residual_m = max_displacement_residual_m.max(residual);
        if current.total_epoch_seconds <= previous.total_epoch_seconds {
            return Err("SOI tick epoch did not advance".into());
        }
    }
    let final_epoch = session.snapshot().total_epoch_seconds;
    let reference = session.snapshot().reference_body_id;
    let seen = session
        .events()
        .iter()
        .filter(|event| expected_event.is_some_and(|expected| event.kind == expected))
        .count();
    if reference != expected_body || expected_event.is_some() && seen != 1 {
        return Err(format!(
            "SOI checkpoint {label} reference or event mismatch: initial={initial_ref} final={reference}, events={seen}"
        ).into());
    }
    println!(
        "SOI_TICK_ACCEPTANCE checkpoint={label} ticks=36 expected_reference={} event_count={seen} wall_ms={:.2} max_tick_ms={largest_tick_ms:.2} max_position_residual_m={max_displacement_residual_m:.6} epoch_delta_s={:.3}",
        expected_body,
        start.elapsed().as_secs_f64() * 1e3,
        final_epoch - start_epoch
    );
    session.close().await;
    let reopened = RocketconSession::load(&dest, id).await?;
    if reopened.snapshot().reference_body_id != expected_body
        || (reopened.snapshot().total_epoch_seconds - final_epoch).abs() > 0.001
    {
        return Err(format!("checkpoint {label} did not persist event state").into());
    }
    reopened.close().await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source_dir = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "saves".into()));
    let working = PathBuf::from(
        std::env::args()
            .nth(2)
            .unwrap_or_else(|| "target/c15-jpl-validation".into()),
    );
    std::fs::create_dir_all(&working)?;
    for (label, expected_body, event) in [
        (
            "europa-entry",
            Uuid::from_u128(EUROPA),
            Some(FlightEventKind::SoiEntry),
        ),
        ("europa-close", Uuid::from_u128(EUROPA), None),
        (
            "europa-exit",
            Uuid::from_u128(JUPITER),
            Some(FlightEventKind::SoiExit),
        ),
    ] {
        verify_one(
            &source_dir.join(format!("c15-jpl-{label}.db")),
            &working,
            label,
            expected_body,
            event,
        )
        .await?;
    }
    println!("C15_JOVIAN_SOI_TICK_PASS");
    Ok(())
}
