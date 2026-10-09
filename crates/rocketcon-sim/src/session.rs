mod snapshot;
mod vehicle_visual;

pub use snapshot::{FlightEvent, FlightEventKind, FlightSnapshot};
pub use vehicle_visual::VehicleVisualComponent;

use crate::{RocketError, RocketResult};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use rocketcon_app::aeroespacial::advance_vehicle_simulation_in_session;
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_db::repositories::vehicle_physical_state;
use rocketcon_db::repositories::vehicle_repository;
use rocketcon_db::tick_transaction::TickTransactionSession;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct RocketconSession {
    pool: SqlitePool,
    tick_session: TickTransactionSession,
    save_path: PathBuf,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    control: VehicleControlInput,
    snapshot: FlightSnapshot,
    vehicle_components: Vec<VehicleVisualComponent>,
    trajectory: Vec<FlightSnapshot>,
    events: Vec<FlightEvent>,
}

impl RocketconSession {
    pub async fn load(path: impl AsRef<Path>, vehicle_id: Uuid) -> RocketResult<Self> {
        let path = path.as_ref();
        if !path.is_file() {
            return Err(RocketError::Generic(format!(
                "save not found: {}",
                path.display()
            )));
        }
        let path = path.canonicalize()?;
        let pool = astronomicon_db::connection::open_pool_path(&path).await?;
        rocketcon_db::save::migrations::run_rocketcon_migrations(&pool).await?;
        let result = Self::from_pool(pool.clone(), path, vehicle_id).await;
        if result.is_err() {
            pool.close().await;
        }
        result
    }

    async fn from_pool(
        pool: SqlitePool,
        save_path: PathBuf,
        vehicle_id: Uuid,
    ) -> RocketResult<Self> {
        let state = vehicle_physical_state::get_by_vehicle_id(&pool, &vehicle_id)
            .await?
            .ok_or_else(|| {
                RocketError::Generic(format!(
                    "physical state for vehicle '{vehicle_id}' not found"
                ))
            })?;
        let universe_epoch = rocketcon_app::universe::resolve_universe_epoch(&pool).await?;
        let environment = rocketcon_app::environment::load_environment_snapshot(
            &pool,
            state.reference_body_id(),
            universe_epoch,
            state.captured_at_epoch(),
        )
        .await?;
        let mut snapshot = FlightSnapshot::from_state(&state);
        let vehicle_components = vehicle_repository::list_components_for_vehicle(&pool, &vehicle_id)
            .await?
            .iter()
            .map(|(entry, record)| VehicleVisualComponent::from_assembly(entry, record))
            .collect();
        let body_position = environment.planet_position.raw();
        snapshot.reference_body_position_m = Some([body_position.0, body_position.1, body_position.2]);
        snapshot.reference_body_radius_m = environment.planet.equatorial_radius().map(|radius| radius.value());
        if let Some(radius) = snapshot.reference_body_radius_m {
            snapshot.altitude_m = snapshot::geometric_altitude(
                snapshot.position_m,
                [body_position.0, body_position.1, body_position.2],
                radius,
            );
        }
        let (_, body_velocity) = rocketcon_app::orbital::soi::resolve_body_state_at_epoch(
            &pool,
            state.reference_body_id(),
            environment.system_id,
            state.captured_total_epoch(),
        ).await?;
        let body_velocity = body_velocity.raw();
        snapshot.reference_speed_m_s = snapshot::relative_speed(
            snapshot.velocity_m_s,
            [body_velocity.0, body_velocity.1, body_velocity.2],
        );
        let tick_session = TickTransactionSession::new(&pool).await?;
        Ok(Self {
            pool,
            tick_session,
            save_path,
            vehicle_id,
            universe_epoch,
            control: VehicleControlInput::new(),
            snapshot: snapshot.clone(),
            vehicle_components,
            trajectory: vec![snapshot],
            events: Vec::new(),
        })
    }

    pub fn apply_control(&mut self, control: VehicleControlInput) {
        self.control = control;
    }

    pub async fn step(&mut self, dt_seconds: f64) -> RocketResult<&FlightSnapshot> {
        if !dt_seconds.is_finite() || dt_seconds <= 0.0 {
            return Err(RocketError::Generic(
                "tick duration must be positive and finite".into(),
            ));
        }
        let report = advance_vehicle_simulation_in_session(
            &self.tick_session,
            self.vehicle_id,
            Duration::new(dt_seconds),
            self.universe_epoch,
            &self.control,
        )
        .await?;
        let next = FlightSnapshot::from_report(&report);
        self.events
            .extend(FlightEvent::between(&self.snapshot, &next));
        self.snapshot = next.clone();
        self.trajectory.push(next);
        Ok(&self.snapshot)
    }

    pub fn snapshot(&self) -> &FlightSnapshot {
        &self.snapshot
    }

    pub fn vehicle_components(&self) -> &[VehicleVisualComponent] {
        &self.vehicle_components
    }

    pub fn trajectory(&self) -> &[FlightSnapshot] {
        &self.trajectory
    }

    pub fn events(&self) -> &[FlightEvent] {
        &self.events
    }

    pub fn save_path(&self) -> &Path {
        &self.save_path
    }

    pub async fn save(&self) -> RocketResult<()> {
        sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
            .fetch_all(&self.pool)
            .await
            .map_err(rocketcon_db::error::RocketDbError::from)?;
        Ok(())
    }

    pub async fn close(self) {
        self.tick_session.close().await;
        self.pool.close().await;
    }
}
