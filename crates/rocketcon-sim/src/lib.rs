pub mod bridge;
pub mod clock;

pub use bridge::*;
pub use clock::*;
pub use rocketcon_app::{RocketError, RocketResult};

use uuid::Uuid;

pub const DEFAULT_TICK_COUNT: u32 = 10_000;
pub const DEFAULT_TICK_SECONDS: f64 = 1.0;

pub fn run() -> RocketResult<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let ctx = astronomicon_app::build_context().await?;

        let planet_id = Uuid::parse_str("4beb55b2-62de-4ec2-abe5-ec00290407f8")?;

        let tick_count = match std::env::var("ROCKETCON_TICK_COUNT") {
            Ok(val) => val
                .parse::<u32>()
                .map_err(|e| RocketError::Generic(format!("invalid ROCKETCON_TICK_COUNT: {}", e)))?,
            Err(_) => DEFAULT_TICK_COUNT,
        };

        let dt_seconds = match std::env::var("ROCKETCON_TICK_SECONDS") {
            Ok(val) => val.parse::<f64>().map_err(|e| {
                RocketError::Generic(format!("invalid ROCKETCON_TICK_SECONDS: {}", e))
            })?,
            Err(_) => DEFAULT_TICK_SECONDS,
        };

        let report = run_bridge_smoke_test(&ctx, planet_id, tick_count, dt_seconds).await?;

        println!("Rocketcon Bridge Smoke Test Report:");
        println!("  Tick Count: {}", report.tick_count);
        println!("  Total Wall Clock: {:.6} s", report.total_wall_clock_seconds);
        println!("  Average Tick: {:.2} ns", report.average_tick_nanos);
        println!("  Last Acceleration: {:?}", report.last_computed_acceleration);
        println!("  Final Total Epoch: {:.2} s", report.final_total_epoch.value());

        Ok(())
    })
}
