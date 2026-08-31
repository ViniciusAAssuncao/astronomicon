use crate::clock::SimulationClock;
use astronomicon_app::AppContext;
use astronomicon_core::units::{AccelerationVector, Duration, Position};
use rocketcon_app::error::{RocketError, RocketResult};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct BridgeSmokeTestReport {
    pub tick_count: u32,
    pub total_wall_clock_seconds: f64,
    pub average_tick_nanos: f64,
    pub last_computed_acceleration: AccelerationVector,
    pub final_total_epoch: Duration,
}

pub async fn run_bridge_smoke_test(
    ctx: &AppContext,
    planet_id: Uuid,
    tick_count: u32,
    dt_seconds: f64,
) -> RocketResult<BridgeSmokeTestReport> {
    let universe_epoch = rocketcon_app::universe::resolve_universe_epoch(ctx.pool()).await?;
    let mut clock = SimulationClock::new(universe_epoch);

    let snapshot = rocketcon_app::environment::load_environment_snapshot(
        ctx.pool(),
        planet_id,
        clock.universe_epoch(),
        clock.at_epoch(),
    )
    .await?;

    let radius = snapshot.planet.equatorial_radius().ok_or_else(|| {
        RocketError::Generic(format!(
            "planet '{}' has no equatorial radius",
            snapshot.planet.id()
        ))
    })?;

    let point = snapshot.planet_position + Position::from_components(radius.value(), 0.0, 0.0);

    let start = std::time::Instant::now();
    let dt = Duration::new(dt_seconds);
    let mut last_acc = AccelerationVector::zero();

    for _ in 0..tick_count {
        clock.tick(dt);
        last_acc =
            rocketcon_core::environment::gravitational_acceleration_at_snapshot(&snapshot, point);
    }

    let elapsed = start.elapsed();
    let total_wall_clock_seconds = elapsed.as_secs_f64();
    let average_tick_nanos = if tick_count > 0 {
        (elapsed.as_nanos() as f64) / (tick_count as f64)
    } else {
        0.0
    };

    Ok(BridgeSmokeTestReport {
        tick_count,
        total_wall_clock_seconds,
        average_tick_nanos,
        last_computed_acceleration: last_acc,
        final_total_epoch: clock.total_epoch(),
    })
}
