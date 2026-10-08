use super::{INITIAL_AT_EPOCH, percentile, sql_trace};
use astronomicon_app::climate::circulation::{
    resolve_wind_profile_at_latitude, resolve_wind_profile_at_latitude_with_context,
    resolve_wind_profile_at_latitude_with_temperature,
};
use astronomicon_app::climate::temperature::{
    AdvectiveTemperatureContext, ClimateBodyInputs, resolve_advective_surface_temperature,
};
use astronomicon_app::ephemeris::resolve_planet_orientation;
use astronomicon_core::units::{Angle, Duration};
use rocketcon_app::aeroespacial::{
    resolve_vehicle_aerodynamics, resolve_vehicle_aerodynamics_profiled, resolve_vehicle_snapshot,
};
use rocketcon_db::repositories::{vehicle, vehicle_physical_state};
use sqlx::SqlitePool;
use std::error::Error;
use std::f64::consts::PI;
use std::time::Instant;
use uuid::Uuid;

pub async fn run(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    planet_id: Uuid,
    epoch: Duration,
    evaluations: usize,
    trace_sql: bool,
    setup_ms: f64,
) -> Result<(), Box<dyn Error>> {
    let at_epoch = Duration::new(INITIAL_AT_EPOCH);
    let state = vehicle_physical_state::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or("missing physical state")?;
    let environment =
        rocketcon_app::environment::load_environment_snapshot(pool, planet_id, epoch, at_epoch)
            .await?;
    let snapshot = resolve_vehicle_snapshot(pool, vehicle_id, epoch, at_epoch).await?;
    let components = vehicle::list_components_for_vehicle(pool, &vehicle_id).await?;
    let position = environment.planet_position.raw();

    let orientation = resolve_planet_orientation(pool, planet_id, epoch, at_epoch).await?;
    let relative_position = state.position().raw() - position;
    let body_position = orientation.inverse().rotate_vector(relative_position);
    let latitude = Angle::new(
        body_position
            .2
            .atan2((body_position.0 * body_position.0 + body_position.1 * body_position.1).sqrt()),
    );
    let local_temperature =
        resolve_advective_surface_temperature(pool, planet_id, latitude, epoch, at_epoch).await?;
    let original_wind =
        resolve_wind_profile_at_latitude(pool, planet_id, latitude, epoch, at_epoch).await?;
    let reused_wind = resolve_wind_profile_at_latitude_with_temperature(
        pool,
        planet_id,
        latitude,
        epoch,
        at_epoch,
        local_temperature,
    )
    .await?;
    if original_wind != reused_wind {
        return Err("wind changed when reusing local temperature".into());
    }
    let climate = AdvectiveTemperatureContext::load(pool, planet_id, epoch, at_epoch).await?;
    for sample_latitude in [
        latitude,
        Angle::new((latitude.value() + PI / 180.0).min(PI / 2.0)),
        Angle::new((latitude.value() - PI / 180.0).max(-PI / 2.0)),
    ] {
        let original = resolve_advective_surface_temperature(
            pool,
            planet_id,
            sample_latitude,
            epoch,
            at_epoch,
        )
        .await?;
        let shared = climate
            .temperature_at_latitude(pool, sample_latitude)
            .await?;
        if original != shared {
            return Err("surface temperature changed with shared climate context".into());
        }
    }
    let context_wind =
        resolve_wind_profile_at_latitude_with_context(pool, &climate, latitude, local_temperature)
            .await?;
    if original_wind != context_wind {
        return Err("wind changed with shared climate context".into());
    }
    let other_epoch = at_epoch + Duration::new(600.0);
    let later_climate =
        AdvectiveTemperatureContext::load(pool, planet_id, epoch, other_epoch).await?;
    for sample_latitude in [Angle::new(-PI / 3.0), Angle::new(PI / 3.0)] {
        let original_temperature = resolve_advective_surface_temperature(
            pool,
            planet_id,
            sample_latitude,
            epoch,
            other_epoch,
        )
        .await?;
        let shared_temperature = later_climate
            .temperature_at_latitude(pool, sample_latitude)
            .await?;
        if original_temperature != shared_temperature {
            return Err("surface temperature changed at another latitude or epoch".into());
        }
        let original_wind =
            resolve_wind_profile_at_latitude(pool, planet_id, sample_latitude, epoch, other_epoch)
                .await?;
        let shared_wind = resolve_wind_profile_at_latitude_with_context(
            pool,
            &later_climate,
            sample_latitude,
            shared_temperature,
        )
        .await?;
        if original_wind != shared_wind {
            return Err("wind changed at another latitude or epoch".into());
        }
    }
    let shared_body = ClimateBodyInputs::load(pool, planet_id).await?;
    for (sample_epoch, separately_loaded) in [(at_epoch, &climate), (other_epoch, &later_climate)] {
        let shared_context =
            AdvectiveTemperatureContext::load_with_body(pool, &shared_body, epoch, sample_epoch)
                .await?;
        for sample_latitude in [Angle::new(-PI / 3.0), latitude, Angle::new(PI / 3.0)] {
            let expected = separately_loaded
                .temperature_at_latitude(pool, sample_latitude)
                .await?;
            let actual = shared_context
                .temperature_at_latitude(pool, sample_latitude)
                .await?;
            if expected != actual {
                return Err("shared body changed temperature across epochs".into());
            }
            let expected_wind = resolve_wind_profile_at_latitude_with_context(
                pool,
                separately_loaded,
                sample_latitude,
                expected,
            )
            .await?;
            let actual_wind = resolve_wind_profile_at_latitude_with_context(
                pool,
                &shared_context,
                sample_latitude,
                actual,
            )
            .await?;
            if expected_wind != actual_wind {
                return Err("shared body changed wind across epochs".into());
            }
        }
    }
    let dry_planet_id = Uuid::parse_str(super::MEROS_ID)?;
    let dry_body = ClimateBodyInputs::load(pool, dry_planet_id).await?;
    for sample_epoch in [at_epoch, other_epoch] {
        let dry_context =
            AdvectiveTemperatureContext::load_with_body(pool, &dry_body, epoch, sample_epoch)
                .await?;
        let sample_latitude = Angle::new(PI / 6.0);
        let expected_temperature = resolve_advective_surface_temperature(
            pool,
            dry_planet_id,
            sample_latitude,
            epoch,
            sample_epoch,
        )
        .await?;
        let actual_temperature = dry_context
            .temperature_at_latitude(pool, sample_latitude)
            .await?;
        if expected_temperature != actual_temperature {
            return Err("shared body changed dry-planet temperature".into());
        }
        let expected_wind = resolve_wind_profile_at_latitude(
            pool,
            dry_planet_id,
            sample_latitude,
            epoch,
            sample_epoch,
        )
        .await?;
        let actual_wind = resolve_wind_profile_at_latitude_with_context(
            pool,
            &dry_context,
            sample_latitude,
            actual_temperature,
        )
        .await?;
        if expected_wind != actual_wind {
            return Err("shared body changed dry-planet wind".into());
        }
    }
    let original_albedo: Option<f64> =
        sqlx::query_scalar("SELECT bond_albedo FROM planets WHERE id = ?")
            .bind(planet_id.to_string())
            .fetch_one(pool)
            .await?;
    let mut refresh_body = ClimateBodyInputs::load(pool, planet_id).await?;
    sqlx::query("UPDATE planets SET bond_albedo = ? WHERE id = ?")
        .bind(original_albedo.unwrap_or(0.3) + 0.01)
        .bind(planet_id.to_string())
        .execute(pool)
        .await?;
    let detected_change = refresh_body.refresh_if_changed(pool).await?;
    sqlx::query("UPDATE planets SET bond_albedo = ? WHERE id = ?")
        .bind(original_albedo)
        .bind(planet_id.to_string())
        .execute(pool)
        .await?;
    let detected_restore = refresh_body.refresh_if_changed(pool).await?;
    let no_further_change = !refresh_body.refresh_if_changed(pool).await?;
    if !(detected_change && detected_restore && no_further_change) {
        return Err("climate body refresh failed to detect a committed change".into());
    }
    if trace_sql {
        sql_trace::enable();
        let before = sql_trace::snapshot();
        let measured_body = ClimateBodyInputs::load(pool, planet_id).await?;
        let after_body = sql_trace::snapshot();
        let measured_climate =
            AdvectiveTemperatureContext::load_with_body(pool, &measured_body, epoch, at_epoch)
                .await?;
        let after_load = sql_trace::snapshot();
        let load_tables = sql_trace::top_tables(8);
        let measured_temperature = measured_climate
            .temperature_at_latitude(pool, latitude)
            .await?;
        let after_temperature = sql_trace::snapshot();
        let _ = resolve_wind_profile_at_latitude_with_context(
            pool,
            &measured_climate,
            latitude,
            measured_temperature,
        )
        .await?;
        let after_wind = sql_trace::snapshot();
        sql_trace::disable();
        println!(
            "  climate_sql: body_reads={} epoch_reads={} latitude_reads={} wind_reads={}",
            after_body.since(before).reads,
            after_load.since(after_body).reads,
            after_temperature.since(after_load).reads,
            after_wind.since(after_temperature).reads,
        );
        for (table, count) in load_tables {
            println!("  climate_load_table {table}: statements={count}");
        }
    }

    let expected = resolve_vehicle_aerodynamics(
        pool,
        &state,
        planet_id,
        position,
        &components,
        snapshot.active_stages(),
        epoch,
        at_epoch,
    )
    .await?;
    if expected.is_none() {
        return Err("atmospheric fixture did not produce aerodynamics".into());
    }

    if trace_sql {
        sql_trace::enable();
    }
    let sql_before = sql_trace::snapshot();
    let mut times = Vec::with_capacity(evaluations.saturating_sub(1));
    let mut stage_times: Vec<(&'static str, Vec<f64>)> = Vec::new();
    let mut first_ms = 0.0;
    let mut sql_after_first = sql_before;
    for index in 0..evaluations {
        let start = Instant::now();
        let (actual, profile) = resolve_vehicle_aerodynamics_profiled(
            pool,
            &state,
            planet_id,
            position,
            &components,
            snapshot.active_stages(),
            epoch,
            at_epoch,
        )
        .await?;
        let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
        if actual != expected {
            return Err(format!("aerodynamic diagnostic changed at evaluation {index}").into());
        }
        if index == 0 {
            first_ms = elapsed_ms;
            sql_after_first = sql_trace::snapshot();
            stage_times = profile
                .stages()
                .iter()
                .map(|stage| {
                    (
                        stage.name,
                        Vec::with_capacity(evaluations.saturating_sub(1)),
                    )
                })
                .collect();
        } else {
            times.push(elapsed_ms);
            for ((_, samples), stage) in stage_times.iter_mut().zip(profile.stages()) {
                samples.push(stage.duration.as_secs_f64() * 1_000.0);
            }
        }
    }
    let sql_after_all = sql_trace::snapshot();
    sql_trace::disable();

    times.sort_by(f64::total_cmp);
    println!(
        "aero: evaluations={evaluations} setup_ms={setup_ms:.2} first_ms={first_ms:.2} median_ms={:.2} p95_ms={:.2} identical_wind=true identical_diagnostic=true",
        percentile(&times, 0.5),
        percentile(&times, 0.95),
    );
    for (name, mut samples) in stage_times {
        samples.sort_by(f64::total_cmp);
        println!(
            "  aero_stage {name}: median_ms={:.2} p95_ms={:.2}",
            percentile(&samples, 0.5),
            percentile(&samples, 0.95),
        );
    }
    if trace_sql {
        let first = sql_after_first.since(sql_before);
        let steady = sql_after_all.since(sql_after_first);
        println!(
            "  aero_sql: first_reads={} first_writes={} steady_reads={} steady_writes={} steady_evaluations={}",
            first.reads,
            first.writes,
            steady.reads,
            steady.writes,
            evaluations.saturating_sub(1),
        );
        for (table, count) in sql_trace::top_tables(12) {
            println!("  aero_sql_table {table}: statements={count}");
        }
    }
    Ok(())
}
