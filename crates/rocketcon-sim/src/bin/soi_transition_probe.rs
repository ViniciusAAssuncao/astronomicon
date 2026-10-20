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
    if option.as_deref() == Some("--c3-only") {
        let reader = ForecastReader::open(path, vehicle_id).await?;
        let atlas = reader.celestial_atlas().await?;
        let start = Instant::now();
        let mut weather_bodies = 0;
        let mut raining = 0;
        for body in &atlas.bodies {
            let now = Instant::now();
            let details = reader.body_details(body.id).await?;
            let weather: Vec<_> = details.facts.iter().filter(|f| f.section == "weather").collect();
            if !weather.is_empty() {
                weather_bodies += 1;
                if weather.iter().any(|f| !f.value.is_finite() || f.value < 0.0) ||
                    !weather.iter().any(|f| f.key == "cloud_base_m") ||
                    !details.text_facts.iter().any(|f| f.section == "weather" && f.key == "cloud_condensable") {
                    return Err(format!("invalid weather for {}", body.name).into());
                }
                if weather.iter().any(|f| f.key == "precipitation_mm_h") { raining += 1; }
            }
            println!("C3_BODY name={} weather={} precipitation={} ms={:.2} condensable={}",
                body.name, !weather.is_empty(),
                weather.iter().find(|f| f.key == "precipitation_mm_h").map_or(0.0, |f| f.value),
                now.elapsed().as_secs_f64() * 1000.0,
                details.text_facts.iter().find(|f| f.key == "cloud_condensable")
                    .map_or("-", |f| f.value.as_str()));
        }
        if weather_bodies == 0 {
            return Err("no C3 meteorological body in Zod".into());
        }
        println!("C3_PASS bodies={} with_weather={} with_precipitation={} elapsed_ms={:.2}",
            atlas.bodies.len(), weather_bodies, raining,
            start.elapsed().as_secs_f64() * 1000.0);
        reader.close().await;
        return Ok(());
    }
    if option.as_deref() == Some("--c2-only") {
        let reader = ForecastReader::open(path, vehicle_id).await?;
        let atlas = reader.celestial_atlas().await?;
        let started = Instant::now();
        let mut with_wind = 0usize;
        for body in &atlas.bodies {
            let begin = Instant::now();
            let details = reader.body_details(body.id).await?;
            let facts: Vec<_> = details.facts.iter().filter(|f| f.section == "climate").collect();
            if !facts.is_empty() {
                with_wind += 1;
                if facts.len() != 14 || facts.iter().any(|f| !f.value.is_finite()) {
                    return Err(format!("invalid wind reference for {}: {} fields", body.name, facts.len()).into());
                }
                for fact in &facts {
                    if fact.key.starts_with("surface_wind_") || fact.key.starts_with("jet_wind_") {
                        if fact.value < 0.0 || fact.value > 2000.0 {
                            return Err(format!("invalid magnitude {}={}", fact.key, fact.value).into());
                        }
                    }
                }
            }
            if body.kind == 0 && !facts.is_empty() {
                return Err("stellar body received wind profile".into());
            }
            println!("C2_BODY name={} wind={} ms={:.2} surface30n={:.2}",
                body.name, !facts.is_empty(), begin.elapsed().as_secs_f64()*1000.0,
                facts.iter().find(|f| f.key == "surface_wind_30n").map_or(0.0, |f| f.value));
        }
        if with_wind != 3 { return Err(format!("expected 3 modelled atmospheres, got {with_wind}").into()); }
        println!("C2_PASS bodies={} winds={} elapsed_ms={:.2}",atlas.bodies.len(), with_wind,
            started.elapsed().as_secs_f64() * 1000.0);
        reader.close().await;
        return Ok(());
    }
    if option.as_deref() == Some("--c1-only") {
        let reader = ForecastReader::open(path, vehicle_id).await?;
        let atlas = reader.celestial_atlas().await?;
        let mut atmospheric = 0usize;
        let mut volcanic = 0usize;
        let started = Instant::now();
        let mut slowest_ms: f64 = 0.0;
        for body in &atlas.bodies {
            let now = Instant::now();
            let details = reader.body_details(body.id).await?;
            let elapsed = now.elapsed().as_secs_f64() * 1000.0;
            slowest_ms = slowest_ms.max(elapsed);
            let atmosphere = details
                .facts
                .iter()
                .any(|f| f.section == "atmosphere" && f.key == "surface_air_density");
            let vulcanism = details
                .facts
                .iter()
                .find(|f| f.section == "volcanism" && f.key == "magma_production_rate");
            if atmosphere {
                atmospheric += 1;
                for key in [
                    "mean_molar_mass",
                    "surface_sound_speed",
                    "atmospheric_scale_height",
                ] {
                    if !details.facts.iter().any(|f| {
                        f.section == "atmosphere"
                            && f.key == key
                            && f.value > 0.0
                            && f.value.is_finite()
                    }) {
                        return Err(format!(
                            "missing atmospheric physical fact: {} {key}",
                            body.name
                        )
                        .into());
                    }
                }
            }
            if let Some(rate) = vulcanism {
                volcanic += 1;
                if !rate.value.is_finite()
                    || rate.value <= 0.0
                    || !details
                        .text_facts
                        .iter()
                        .any(|f| f.section == "volcanism" && f.key == "volcanic_style")
                {
                    return Err(format!("invalid volcanic diagnostic: {}", body.name).into());
                }
            }
            if body.kind == 0 && (atmosphere || vulcanism.is_some()) {
                return Err(format!("invalid stellar diagnostic: {}", body.name).into());
            }
            println!(
                "C1_BODY name={} atmosphere={} volcanism={} rate_kg_s={:.6e} elapsed_ms={:.2}",
                body.name,
                atmosphere,
                vulcanism.is_some(),
                vulcanism.map_or(0.0, |v| v.value),
                elapsed
            );
        }
        if atmospheric < 3 {
            return Err(format!("incomplete atmospheric coverage: {atmospheric}").into());
        }
        println!(
            "C1_PASS bodies={} atmospheres={} volcanic={} total_ms={:.2} slowest_ms={:.2}",
            atlas.bodies.len(),
            atmospheric,
            volcanic,
            started.elapsed().as_secs_f64() * 1000.0,
            slowest_ms
        );
        reader.close().await;
        return Ok(());
    }
    if option.as_deref() == Some("--b2a-only") {
        let reader = ForecastReader::open(path, vehicle_id).await?;
        let atlas = reader.celestial_atlas().await?;
        let mut radiation = 0;
        let mut magnetic = 0;
        let mut surface = 0;
        let mut area = 0;
        let mut cores = 0;
        let mut tectonic = 0;
        let mut terrestrial = 0;
        let now = Instant::now();
        for body in &atlas.bodies {
            let details = reader.body_details(body.id).await?;
            let has = |section: &str, key: &str| details.facts.iter()
                .find(|fact| fact.section == section && fact.key == key)
                .is_some_and(|fact| fact.value.is_finite());
            radiation += usize::from(has("radiation", "stellar_particle_flux"));
            magnetic += usize::from(has("magnetic", "magnetic_equatorial_t"));
            surface += usize::from(has("environment", "surface_mean_k"));
            area += usize::from(has("general", "surface_area_m2"));
            cores += usize::from(has("interior", "core_radius_m"));
            tectonic += usize::from(details.text_facts.iter().any(|fact|
                fact.section == "tectonics" && fact.key == "tectonic_regime"));
            terrestrial += usize::from(has("environment", "earth_similarity"));
            if details.facts.iter().any(|fact| fact.key == "diagnostic_epoch")
                || details.text_facts.iter().any(|fact| fact.key == "reference_model") {
                return Err("developer-only fields were published".into());
            }
        }
        if area < 6 || cores < 3 || tectonic < 2 || terrestrial < 3 {
            return Err(format!("missing geoscience facts: area={area} cores={cores} tectonic={tectonic} esi={terrestrial}").into());
        }
        if radiation < 3 || magnetic == 0 || surface < 3 {
            return Err(format!("insufficient environment coverage: radiation={radiation} magnetic={magnetic} surface={surface}").into());
        }
        println!("GEOSCIENCE_PASS bodies={} radiation={} magnetic={} surface={} area={} cores={} tectonic={} esi={} elapsed_ms={:.2}",
            atlas.bodies.len(), radiation, magnetic, surface, area, cores,
            tectonic, terrestrial, now.elapsed().as_secs_f64() * 1000.0);
        reader.close().await;
        return Ok(());
    }
    if option.as_deref() == Some("--details-only") {
        let reader = ForecastReader::open(path, vehicle_id).await?;
        let atlas = reader.celestial_atlas().await?;
        let mut moons = 0;
        let mut stars = 0;
        let mut io_tidal_power = None;
        for body in &atlas.bodies {
            let details = reader.body_details(body.id).await?;
            if details.id != body.id {
                return Err("body details id mismatch".into());
            }
            if body.kind == 2 {
                moons += 1;
                if details.gravity_m_s2.is_none_or(|value| !value.is_finite() || value <= 0.0) ||
                    details.orbital_period_seconds.is_none() {
                    return Err(format!("incomplete moon facts: {}", body.name).into());
                }
            }
            if body.name == "Io" {
                io_tidal_power = details.facts.iter()
                    .find(|fact| fact.section == "tides" && fact.key == "tidal_power_w")
                    .map(|fact| fact.value);
            }
            if body.kind == 0 {
                stars += 1;
                if details.atmosphere.is_some() || details.hydrosphere.is_some() {
                    return Err("star was misclassified with planetary environment".into());
                }
            }
            if let Some(atmosphere) = &details.atmosphere {
                if atmosphere.pressure_pa < 1.0 ||
                    atmosphere.gases.iter().any(|gas| gas.percentage < 0.0 ||
                        !gas.percentage.is_finite()) {
                    return Err("invalid atmospheric facts".into());
                }
            }
        }
        if moons < 4 || stars == 0 {
            return Err("unexpected missing Jovian catalogue bodies".into());
        }
        if !io_tidal_power.is_some_and(|value| value.is_finite() && value > 0.0) {
            return Err("Io did not receive calculated tidal power".into());
        }
        println!("BODY_DETAILS_PASS bodies={} moons={} stars={} io_tidal_power_w={:.3e}",
            atlas.bodies.len(), moons, stars, io_tidal_power.unwrap());
        reader.close().await;
        return Ok(());
    }
    if option.as_deref() == Some("--atlas-only") {
        let reader = ForecastReader::open(path, vehicle_id).await?;
        let atlas = reader.celestial_atlas().await?;
        if atlas.bodies.is_empty() ||
            !atlas.bodies.iter().any(|b| b.id == atlas.reference_body_id &&
                b.relative_position_m.iter().all(|v| v.abs() < 1.0)) ||
            atlas.bodies.iter().any(|b| b.relative_position_m.iter().any(|v| !v.is_finite())) {
            return Err("invalid celestial atlas".into());
        }
        let mut kinds = [0usize; 6];
        let mut orbits = 0usize;
        let mut soi_count = 0usize;
        let mut moon_soi_count = 0usize;
        for body in &atlas.bodies {
            kinds[body.kind as usize] += 1;
            if let Some(soi) = body.soi_radius_m {
                if !soi.is_finite() || soi <= 0.0 || body.kind == 5 {
                    return Err(format!("invalid catalog SOI: {}", body.name).into());
                }
                soi_count += 1;
                if body.kind == 2 {
                    moon_soi_count += 1;
                }
            } else if body.kind == 2 && body.mass_kg.is_some() &&
                body.parent_id.is_some() && !body.orbit_points_m.is_empty() {
                return Err(format!("catalog moon missing SOI: {}", body.name).into());
            }
            if !body.orbit_points_m.is_empty() {
                orbits += 1;
                let first = body.orbit_points_m.first().unwrap();
                let last = body.orbit_points_m.last().unwrap();
                let gap = first.iter().zip(last).map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>().sqrt();
                if gap > 1.0 || body.orbit_points_m.len() != 129 ||
                    body.orbit_points_m.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(format!("invalid orbit catalog curve: {}", body.name).into());
                }
                let mut nearest = f64::INFINITY;
                let mut largest_step: f64 = 0.0;
                for pair in body.orbit_points_m.windows(2) {
                    let edge: [f64; 3] = std::array::from_fn(|i| pair[1][i] - pair[0][i]);
                    let delta: [f64; 3] = std::array::from_fn(|i| body.relative_position_m[i] - pair[0][i]);
                    let edge_sq = edge.iter().map(|v| v * v).sum::<f64>();
                    let t = if edge_sq > 0.0 {
                        (delta.iter().zip(edge).map(|(v, e)| v * e).sum::<f64>() / edge_sq)
                            .clamp(0.0, 1.0)
                    } else { 0.0 };
                    let distance = (0..3).map(|i| (delta[i] - t * edge[i]).powi(2))
                        .sum::<f64>().sqrt();
                    nearest = nearest.min(distance);
                    largest_step = largest_step.max(edge_sq.sqrt());
                }
                if nearest > largest_step * 0.1 + 1.0 {
                    return Err(format!("catalog orbit misses {}: distance={:.3} m / step={:.3} m",
                        body.name, nearest, largest_step).into());
                }
            }
        }
        if orbits < 4 || moon_soi_count == 0 {
            return Err("atlas missing configured orbits or moon SOIs".into());
        }
        println!("ATLAS_PASS bodies={} orbits={} soi={} moon_soi={} kinds={:?} epoch={:.3}",
            atlas.bodies.len(), orbits, soi_count, moon_soi_count, kinds, atlas.epoch_seconds);
        reader.close().await;
        return Ok(());
    }
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
        if !completed.continuation_points_m.is_empty() {
            let end = completed.future_arcs.last()
                .and_then(|arc| arc.points_m.last())
                .ok_or("no forecast endpoint for continuation")?;
            let beginning = completed.continuation_points_m.first().unwrap();
            let gap = end.iter().zip(beginning)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            if !gap.is_finite() || gap > 1.0 {
                return Err(format!("forecast continuation gap {gap} m").into());
            }
        }
        println!(
            "FORECAST_PROGRESS_PASS windows={} continuation_vertices={}",
            partials.len(), completed.continuation_points_m.len()
        );
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
        if preview.orbit_classification == 3 {
            let first = preview.future_arcs.first().ok_or("missing current patch")?;
            let past = &first.reference_points_m;
            if past.len() < 64 || past.iter().flatten().any(|value| !value.is_finite()) {
                return Err("missing or invalid inbound osculating patch".into());
            }
            let start = first.points_m.first().ok_or("missing craft anchor")?;
            let end = past.last().ok_or("missing osculating endpoint")?;
            let deviation = start.iter().zip(end.iter())
                .map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
            if deviation > 0.01 {
                return Err(format!("retroconic is disconnected from craft: {deviation} m").into());
            }
            println!("FORECAST_INBOUND_OSCULATING_PASS vertices={} anchor_delta_m={deviation:.6}", past.len());
        }
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
