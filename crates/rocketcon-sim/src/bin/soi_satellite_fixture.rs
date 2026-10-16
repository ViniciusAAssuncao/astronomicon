use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::{OrbitalParent, Planet};
use astronomicon_core::units::{Position, VelocityVector};
use astronomicon_db::repositories::planet_repository;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::orbital::sphere_of_influence::laplace_sphere_of_influence_radius;
use rocketcon_db::repositories::vehicle_physical_state;
use rocketcon_sim::{FlightEventKind, RocketconSession};
use std::error::Error;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let source = Path::new("saves/c14-outbound.db");
    let destination = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "saves/c14-satellite-inbound.db".into());
    let target_path = PathBuf::from(destination);
    let target = target_path.as_path();
    if target.exists() {
        return Err("destination save already exists; refusing to overwrite it".into());
    }
    std::fs::copy(source, target)?;
    let url = format!("sqlite://{}", target.display());
    let pool = astronomicon_db::connection::open_pool(&url).await?;
    let vehicle_id = Uuid::parse_str("715c3661-e69f-475c-8c78-64927ae12b01")?;
    let initial = vehicle_physical_state::get_by_vehicle_id(&pool, &vehicle_id)
        .await?
        .ok_or("vehicle physical state missing")?;
    let env = rocketcon_app::environment::load_environment_snapshot(
        &pool,
        initial.reference_body_id(),
        initial.captured_universe_epoch(),
        initial.captured_at_epoch(),
    )
    .await?;
    let rows = planet_repository::list_by_system(&pool, &env.system_id).await?;
    let planets = rows
        .into_iter()
        .map(Planet::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    let moon = planets
        .iter()
        .find(|planet| matches!(planet.orbital_parent(), OrbitalParent::Planet(_)))
        .ok_or("catalog has no satellite")?;
    let OrbitalParent::Planet(parent_id) = moon.orbital_parent() else {
        return Err("satellite parent is not a planet".into());
    };
    let parent = planets
        .iter()
        .find(|planet| planet.id() == parent_id)
        .ok_or("satellite primary missing")?;
    let radius = laplace_sphere_of_influence_radius(
        moon.orbital_elements()
            .ok_or("satellite elements missing")?
            .semi_major_axis(),
        moon.mass(),
        parent.mass(),
    )
    .value();
    let ephemeris = EphemerisCatalog::load(&pool, env.system_id).await?;
    let states = ephemeris.states(env.captured_total_epoch())?;
    let moon_state = states
        .get(&moon.id())
        .ok_or("satellite ephemeris missing")?;
    let parent_state = states.get(&parent_id).ok_or("parent ephemeris missing")?;
    let radial = moon_state.position.raw() - parent_state.position.raw();
    let direction = radial / radial.magnitude();
    let initial_position =
        Position::from_raw(moon_state.position.raw() + direction * (radius * (1.0 + 1.0e-5)));
    let initial_velocity = VelocityVector::from_raw(moon_state.velocity.raw() - direction * 800.0);
    let state = VehiclePhysicalState::new_with_max_q(
        vehicle_id,
        initial_position,
        initial_velocity,
        initial.orientation(),
        initial.angular_velocity(),
        parent_id,
        initial.captured_universe_epoch(),
        initial.captured_at_epoch(),
        initial.max_dynamic_pressure(),
        initial.max_dynamic_pressure_epoch(),
    )?;
    vehicle_physical_state::upsert(&pool, &state).await?;
    println!(
        "SATELLITE={} PRIMARY={} SOI_RADIUS_M={radius:.3}",
        moon.name(),
        parent.name()
    );
    let moon_id = moon.id();
    pool.close().await;
    let verify_path = Path::new("saves/c14-satellite-verify-v2.db");
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{}", verify_path.display(), suffix));
    }
    std::fs::copy(target, verify_path)?;
    let mut session = RocketconSession::load(verify_path, vehicle_id).await?;
    println!(
        "INITIAL_REF={} X={}",
        session.snapshot().reference_body_id,
        session.snapshot().position_m[0]
    );
    let mut crossing = None;
    for step in 0..240 {
        let state = session.step(0.02).await?;
        if state.reference_body_id == moon_id {
            crossing = Some(step);
            break;
        }
    }
    let step = crossing.ok_or("satellite SOI entry not detected")?;
    let entries = session
        .events()
        .iter()
        .filter(|event| event.kind == FlightEventKind::SoiEntry)
        .count();
    if entries != 1 {
        return Err(format!("expected one SOI entry, observed {entries}").into());
    }
    session.close().await;
    let reopened = RocketconSession::load(verify_path, vehicle_id).await?;
    if reopened.snapshot().reference_body_id != moon_id {
        return Err("satellite reference did not persist".into());
    }
    reopened.close().await;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(format!("{}{}", verify_path.display(), suffix));
    }
    println!("SATELLITE_SOI_PASS step={step} persisted=true");
    Ok(())
}
