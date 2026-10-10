use rocketcon_core::domain::VehicleControlInput;
use rocketcon_sim::RocketconSession;
use std::error::Error;
use std::path::PathBuf;
use uuid::Uuid;

struct Args {
    save: PathBuf,
    vehicle_id: Uuid,
    ticks: usize,
    dt_seconds: f64,
    pitch: f64,
    yaw: f64,
    roll: f64,
}

fn parse_args() -> Result<Args, Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage =
        "usage: flight_session SAVE_PATH VEHICLE_UUID [TICKS] [DT_SECONDS] [PITCH] [YAW] [ROLL]";
    let save = PathBuf::from(args.next().ok_or(usage)?);
    let vehicle_id = args.next().ok_or(usage)?.parse()?;
    let ticks = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(10);
    let dt_seconds = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(0.02);
    let pitch = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(0.0);
    let yaw = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(0.0);
    let roll = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(0.0);
    if args.next().is_some() || ticks == 0 {
        return Err(usage.into());
    }
    Ok(Args {
        save,
        vehicle_id,
        ticks,
        dt_seconds,
        pitch,
        yaw,
        roll,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = parse_args()?;
    let mut session = RocketconSession::load(&args.save, args.vehicle_id).await?;
    session.apply_control(
        VehicleControlInput::new().with_pitch_yaw_roll(args.pitch, args.yaw, args.roll),
    );
    println!(
        "save={} vehicle={}",
        session.save_path().display(),
        args.vehicle_id
    );
    for tick in 1..=args.ticks {
        let snapshot = session.step(args.dt_seconds).await?;
        println!(
            "tick={tick} epoch_s={:.3} speed_m_s={:.3} relative_speed_m_s={:?} vertical_speed_m_s={:?} altitude_m={:?} mach={:?} g={:?} position_m={:?}",
            snapshot.total_epoch_seconds,
            snapshot.speed_m_s,
            snapshot.reference_speed_m_s,
            snapshot.reference_vertical_speed_m_s,
            snapshot.altitude_m,
            snapshot.mach,
            snapshot.total_g_load,
            snapshot.position_m,
        );
    }
    session.save().await?;
    println!(
        "trajectory_samples={} events={:?}",
        session.trajectory().len(),
        session.events()
    );
    session.close().await;
    Ok(())
}
