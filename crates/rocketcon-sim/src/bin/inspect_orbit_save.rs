use rocketcon_sim::RocketconSession;
use std::error::Error;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let session = RocketconSession::load(
        "saves/flight-analysis.db".as_ref(),
        Uuid::parse_str("715c3661-e69f-475c-8c78-64927ae12b01")?,
    ).await?;
    let snapshot = session.snapshot();
    let preview = session.orbit_preview().await?;
    println!("epoch={} altitude={:?} speed={:?} vertical={:?} fuel={:?} engine={:?}",
        snapshot.total_epoch_seconds, snapshot.altitude_m, snapshot.reference_speed_m_s,
        snapshot.reference_vertical_speed_m_s, snapshot.fuel_reserves,
        snapshot.main_engine_loads);
    println!("pe_alt={:?} ap_alt={:?} period={:?} pe_event={:?} ap_event={:?} impact={:?} horizon={}",
        preview.periapsis_altitude_m, preview.apoapsis_altitude_m, preview.period_seconds,
        preview.next_periapsis.as_ref().map(|event| event.time_to_seconds),
        preview.next_apoapsis.as_ref().map(|event| event.time_to_seconds),
        preview.impact_epoch_seconds, preview.horizon_seconds);
    session.close().await;
    Ok(())
}
