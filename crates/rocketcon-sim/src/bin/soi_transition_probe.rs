use rocketcon_sim::{FlightEventKind, ForecastReader, RocketconSession};
use std::error::Error;
use std::path::Path;
use std::time::Instant;
use uuid::Uuid;

const VEHICLE_ID: &str = "715c3661-e69f-475c-8c78-64927ae12b01";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("save path required")?;
    let option = args.next();
    let powered = option.as_deref() == Some("--powered");
    let tangent = option.as_deref() == Some("--tangent");
    let path = Path::new(&path);
    let vehicle_id = Uuid::parse_str(VEHICLE_ID)?;
    if option.as_deref() == Some("--progress-only") {
        let reader = ForecastReader::open(path, vehicle_id).await?;
        let mut partials = Vec::new();
        let completed = reader
            .preview_cancelled(false, &mut || false, &mut |partial| {
                partials.push(partial);
            })
            .await?;
        if partials.is_empty() {
            return Err("forecast did not publish progressive windows".into());
        }
        let mut previous = 0.0;
        for partial in &partials {
            if partial.forecast_completed_seconds <= previous
                || partial.forecast_completed_seconds > completed.forecast_completed_seconds
                || !partial.forecast_completed_seconds.is_finite()
            {
                return Err("progressive horizon did not increase".into());
            }
            previous = partial.forecast_completed_seconds;
            for arcs in partial.future_arcs.windows(2) {
                let first = arcs[0].points_m.last().ok_or("empty progressive arc")?;
                let next = arcs[1].points_m.first().ok_or("empty progressive arc")?;
                let delta = first
                    .iter()
                    .zip(next)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if delta > 1.0 {
                    return Err("progressive SOI geometry discontinuity".into());
                }
            }
            println!(
                "FORECAST_PROGRESS reached={:.3} target={:.3} arcs={} complete={}",
                partial.forecast_completed_seconds,
                partial.forecast_target_seconds,
                partial.future_arcs.len(),
                partial.forecast_complete
            );
        }
        if (previous - completed.forecast_completed_seconds).abs() > 1e-6 {
            return Err("last progressive forecast does not match final horizon".into());
        }
        println!("FORECAST_PROGRESS_PASS windows={}", partials.len());
        reader.close().await;
        return Ok(());
    }
    let mut session = RocketconSession::load(path, vehicle_id).await?;
    let initial = session.snapshot().clone();
    let initial_body = initial.reference_body_id;
    let forecast_started = Instant::now();
    let preview = session.orbit_preview().await?;
    println!(
        "FORECAST arcs={} milliseconds={:.3} rendered_seconds={:.3}",
        preview.future_arcs.len(),
        forecast_started.elapsed().as_secs_f64() * 1000.0,
        preview.rendered_horizon_seconds,
    );
    for arc in &preview.future_arcs {
        let distance = match (arc.points_m.first(), arc.points_m.last()) {
            (Some(start), Some(end)) => start
                .iter()
                .zip(end.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt(),
            _ => 0.0,
        };
        println!(
            "FORECAST_ARC ref={} vertices={} from_s={:.3} to_s={:.3} displacement_m={:.3}",
            arc.reference_body_id,
            arc.points_m.len(),
            arc.start_seconds,
            arc.end_seconds,
            distance,
        );
    }
    for event in &preview.forecast_events {
        println!(
            "FORECAST_EVENT kind={} body={} seconds={:.3} position={:?}",
            event.kind, event.body_name, event.seconds, event.position_m,
        );
        if event.kind == 2 {
            let residual = preview
                .forecast_bodies
                .iter()
                .filter(|body| body.name == event.body_name)
                .map(|body| {
                    let distance = body
                        .position_m
                        .iter()
                        .zip(event.position_m.iter())
                        .map(|(a, b)| (a - b).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    (distance - body.radius_m).abs()
                })
                .fold(f64::INFINITY, f64::min);
            println!("IMPACT_SURFACE_RESIDUAL_M={residual:.6}");
            if residual > 1.0 {
                return Err("impact marker does not meet the displayed body surface".into());
            }
        }
    }
    if option.as_deref() == Some("--preview-only") {
        session.close().await;
        return Ok(());
    }
    for pair in preview.future_arcs.windows(2) {
        let before = pair[0].points_m.last().ok_or("empty forecast arc")?;
        let after = pair[1].points_m.first().ok_or("empty forecast arc")?;
        let separation = before
            .iter()
            .zip(after.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        println!(
            "FORECAST_BOUNDARY ref={} -> {} at={:.3} delta_m={:.6}",
            pair[0].reference_body_id, pair[1].reference_body_id, pair[0].end_seconds, separation,
        );
        if separation > 1.0 {
            return Err("forecast arc discontinuity".into());
        }
    }
    if option.as_deref() == Some("--forecast-only") {
        if preview.future_arcs.len() < 2 {
            return Err("expected two or more future orbit arcs".into());
        }
        session.close().await;
        return Ok(());
    }
    let entering = preview.soi.is_none();
    if powered {
        let id = initial
            .main_engine_loads
            .first()
            .ok_or("main engine missing")?
            .0
            .parse()?;
        session.set_main_engine_load(id, 1.0).await?;
    }
    let mut crossings = 0;
    let mut star_ticks = Vec::new();
    let mut planet_ticks = Vec::new();
    for step in 0..240 {
        let old = session.snapshot().clone();
        let started = Instant::now();
        let current = session.step(0.02).await?.clone();
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let old_is_stellar = if entering {
            old.reference_body_id == initial_body
        } else {
            old.reference_body_id != initial_body
        };
        if old_is_stellar {
            star_ticks.push(elapsed_ms);
        } else {
            planet_ticks.push(elapsed_ms);
        }
        if current.reference_body_id == old.reference_body_id {
            continue;
        }
        crossings += 1;
        let crossing_event = session
            .events()
            .iter()
            .rev()
            .find(|event| {
                matches!(
                    event.kind,
                    FlightEventKind::SoiEntry | FlightEventKind::SoiExit,
                )
            })
            .ok_or("SOI transition has no flight event")?;
        if crossing_event.total_epoch_seconds < old.total_epoch_seconds
            || crossing_event.total_epoch_seconds > current.total_epoch_seconds
        {
            return Err("SOI event epoch is outside the integration step".into());
        }
        println!(
            "EVENT_IN_STEP offset_s={:.6} step_s={:.6}",
            crossing_event.total_epoch_seconds - old.total_epoch_seconds,
            current.total_epoch_seconds - old.total_epoch_seconds,
        );
        let position_delta = old
            .position_m
            .iter()
            .zip(current.position_m.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        let velocity_delta = old
            .velocity_m_s
            .iter()
            .zip(current.velocity_m_s.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        if position_delta > 1_500.0 || velocity_delta > 120.0 {
            return Err(format!(
                "discontinuity: position={position_delta} velocity={velocity_delta}"
            )
            .into());
        }
        println!(
            "CROSSING step={step} before={} after={} position_delta_m={position_delta:.6} velocity_delta_m_s={velocity_delta:.6}",
            old.reference_body_id, current.reference_body_id
        );
        let preview = session.orbit_preview().await?;
        if preview.reference_body_id != current.reference_body_id
            || preview.soi.is_some() != entering
        {
            return Err("preview reference or SOI classification is inconsistent".into());
        }
        println!(
            "PREVIEW points={} eccentricity={:?} horizon_seconds={:.1}",
            preview.relative_points_m.len(),
            preview.eccentricity,
            preview.rendered_horizon_seconds
        );
    }
    star_ticks.sort_by(f64::total_cmp);
    planet_ticks.sort_by(f64::total_cmp);
    println!(
        "C10 RELEASE TIMINGS p50_star_ms={:.3} samples={} p50_planet_ms={:.3} samples={}",
        star_ticks.get(star_ticks.len() / 2).copied().unwrap_or(0.0),
        star_ticks.len(),
        planet_ticks
            .get(planet_ticks.len() / 2)
            .copied()
            .unwrap_or(0.0),
        planet_ticks.len()
    );
    if tangent {
        if crossings != 0 || session.snapshot().reference_body_id != initial_body {
            return Err("tangential flyby switched reference spuriously".into());
        }
        session.close().await;
        let mut reopened = RocketconSession::load(path, vehicle_id).await?;
        reopened.step(0.02).await?;
        if reopened.snapshot().reference_body_id != initial_body {
            return Err("tangential reference changed after reopening".into());
        }
        reopened.close().await;
        println!("C10 TANGENTIAL PASS (no spurious SOI transition)");
        return Ok(());
    }
    if crossings != 1 {
        return Err(format!("expected exactly one SOI crossing, received {crossings}").into());
    }
    let expected = if entering {
        FlightEventKind::SoiEntry
    } else {
        FlightEventKind::SoiExit
    };
    if session
        .events()
        .iter()
        .filter(|event| event.kind == expected)
        .count()
        != 1
    {
        return Err("confirmed SOI event missing or duplicated".into());
    }
    let final_state = session.snapshot().clone();
    if final_state.reference_body_id == initial_body {
        return Err("reference not changed".into());
    }
    session.close().await;
    let mut reopened = RocketconSession::load(path, vehicle_id).await?;
    if reopened.snapshot().reference_body_id != final_state.reference_body_id {
        return Err("reference body not persisted".into());
    }
    reopened.step(0.02).await?;
    if reopened.snapshot().reference_body_id != final_state.reference_body_id {
        return Err("reference body changed unexpectedly after reopening".into());
    }
    reopened.close().await;
    println!(
        "C10 PASS entering={entering} powered={powered} final_ref={} elapsed={:.3}",
        final_state.reference_body_id,
        final_state.total_epoch_seconds - initial.total_epoch_seconds
    );
    Ok(())
}
