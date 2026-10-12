use rocketcon_sim::RocketconSession;
use std::error::Error;
use std::path::PathBuf;
use std::time::Instant;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .ok_or("usage: nodal_dynamics_probe SAVE UUID [TICKS]")?,
    );
    let vehicle_id: Uuid = args.next().ok_or("vehicle uuid required")?.parse()?;
    let ticks: usize = args
        .next()
        .map(|text| text.parse())
        .transpose()?
        .unwrap_or(40);
    if args.next().is_some() || ticks == 0 || ticks > 2000 {
        return Err("invalid tick count or extra arguments".into());
    }
    let mut session = RocketconSession::load(&path, vehicle_id).await?;
    let original = session.orbit_preview().await?;
    let starting_epoch = session.snapshot().total_epoch_seconds;
    let started = Instant::now();
    for tick in 0..ticks {
        session
            .step(0.02)
            .await
            .map_err(|error| format!("tick {}: {}", tick + 1, error))?;
    }
    let tick_elapsed = started.elapsed();
    let after = session
        .orbit_preview()
        .await
        .map_err(|error| format!("preview after {} ticks: {}", ticks, error))?;
    let ending_epoch = session.snapshot().total_epoch_seconds;
    if (ending_epoch - starting_epoch - ticks as f64 * 0.02).abs() > 0.002 {
        return Err(format!("simulation epoch diverged: initial={starting_epoch:.9} final={ending_epoch:.9} expected_delta={:.9} actual_delta={:.9}", ticks as f64 * 0.02, ending_epoch - starting_epoch).into());
    }
    if after.reference_body_id != original.reference_body_id {
        return Err("reference body changed during short nodal test".into());
    }
    for n in [
        after.nodes.next_ascending.as_ref(),
        after.nodes.next_descending.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if !n.time_to_seconds.is_finite()
            || n.time_to_seconds < 0.0
            || !n.relative_position_m.iter().all(|v| v.is_finite())
        {
            return Err("invalid node after ticks".into());
        }
    }
    let initial_i = original
        .nodes
        .inclination_rad
        .ok_or("initial inclination is unavailable")?;
    let final_i = after
        .nodes
        .inclination_rad
        .ok_or("final inclination is unavailable")?;
    if (initial_i - final_i).abs() > 0.01 {
        return Err("unexpected inclination variation in short coast".into());
    }
    if let (Some(before), Some(now)) = (&original.equatorial, &after.equatorial) {
        let b = before.nodes.inclination_rad.ok_or("equatorial inclination missing before")?;
        let a = now.nodes.inclination_rad.ok_or("equatorial inclination missing after")?;
        if (b - a).abs() > 0.01 {
            return Err("unexpected equatorial inclination variation".into());
        }
        for (past, future) in [
            (&before.nodes.next_ascending, &now.nodes.next_ascending),
            (&before.nodes.next_descending, &now.nodes.next_descending),
        ] {
            if let (Some(past), Some(future)) = (past.as_ref(), future.as_ref()) {
                let drift = (past.time_to_seconds - future.time_to_seconds) - ticks as f64 * 0.02;
                if drift.abs() > 5.0 {
                    return Err(format!("unexpected equatorial node drift: {drift}").into());
                }
            }
        }
        println!(
            "equatorial_inclination_start_deg={:.6} equatorial_inclination_end_deg={:.6} equatorial_NA_start_s={:?} equatorial_NA_end_s={:?} equatorial_ND_start_s={:?} equatorial_ND_end_s={:?}",
            b.to_degrees(), a.to_degrees(),
            before.nodes.next_ascending.as_ref().map(|n| n.time_to_seconds),
            now.nodes.next_ascending.as_ref().map(|n| n.time_to_seconds),
            before.nodes.next_descending.as_ref().map(|n| n.time_to_seconds),
            now.nodes.next_descending.as_ref().map(|n| n.time_to_seconds));
    }
    println!(
        "ticks={ticks} total_tick_ms={:.2} mean_tick_ms={:.3} inclination_start_deg={:.5} inclination_end_deg={:.5} NA_before_s={:?} NA_after_s={:?} ND_before_s={:?} ND_after_s={:?}",
        tick_elapsed.as_secs_f64() * 1000.0,
        tick_elapsed.as_secs_f64() * 1000.0 / ticks as f64,
        initial_i.to_degrees(),
        final_i.to_degrees(),
        original
            .nodes
            .next_ascending
            .as_ref()
            .map(|n| n.time_to_seconds),
        after
            .nodes
            .next_ascending
            .as_ref()
            .map(|n| n.time_to_seconds),
        original
            .nodes
            .next_descending
            .as_ref()
            .map(|n| n.time_to_seconds),
        after
            .nodes
            .next_descending
            .as_ref()
            .map(|n| n.time_to_seconds),
    );
    session.close().await;
    Ok(())
}
