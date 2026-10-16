use rocketcon_sim::RocketconSession;
use std::error::Error;
use std::path::Path;
use std::time::Instant;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let file = std::env::args()
        .nth(1)
        .ok_or("expected test-copy save path")?;
    let ticks: usize = std::env::args()
        .nth(2)
        .map(|n| n.parse())
        .transpose()?
        .unwrap_or(10);
    if ticks == 0 || ticks > 5000 {
        return Err("tick count must be from 1 to 5000".into());
    }
    let vehicle = Uuid::parse_str("715c3661-e69f-475c-8c78-64927ae12b01")?;
    let mut session = RocketconSession::load(Path::new(&file), vehicle).await?;
    println!(
        "C11_SAVE={} REF={}",
        file,
        session.snapshot().reference_body_id
    );
    match session.orbit_preview().await {
        Ok(p) => println!(
            "PREVIEW points={} end={:?} time={:.3}",
            p.relative_points_m.len(),
            p.end_reason,
            p.rendered_horizon_seconds
        ),
        Err(e) => println!("PREVIEW_ERROR={e}"),
    }
    let mut reference = session.snapshot().reference_body_id;
    let mut transitions = 0;
    let mut tick_times_ms = Vec::with_capacity(ticks);
    for i in 0..ticks {
        let began = Instant::now();
        let snapshot = session.step(0.02).await?;
        tick_times_ms.push(began.elapsed().as_secs_f64() * 1000.0);
        if snapshot.reference_body_id != reference {
            transitions += 1;
            reference = snapshot.reference_body_id;
            println!("C11_SAVED_SOI_TRANSITION tick={} ref={}", i, reference);
        }
        if i == 0 || i + 1 == ticks {
            println!(
                "TICK={} ref={} t={:.3}",
                i, snapshot.reference_body_id, snapshot.total_epoch_seconds
            );
        }
    }
    tick_times_ms.sort_by(|a, b| a.total_cmp(b));
    let index = |quantile: f64| ((ticks - 1) as f64 * quantile).ceil() as usize;
    println!(
        "C11_TICK_PROFILE n={} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3}",
        ticks,
        tick_times_ms[index(0.5)],
        tick_times_ms[index(0.95)],
        tick_times_ms[index(0.99)],
        tick_times_ms[ticks - 1]
    );
    session.close().await;
    println!("C11_SAVE_PASS ticks={} transitions={}", ticks, transitions);
    Ok(())
}
