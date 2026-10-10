mod snapshot;
mod vehicle_visual;
mod engine_control;

pub use snapshot::{FlightEvent, FlightEventKind, FlightSnapshot};
pub use vehicle_visual::VehicleVisualComponent;

use crate::{RocketError, RocketResult};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use rocketcon_app::aeroespacial::advance_vehicle_simulation_in_session;
use rocketcon_app::aeroespacial::vehicle::resolve_vehicle_snapshot;
use rocketcon_core::domain::{ComponentDetails, VehicleControlInput};
use engine_control::MainEngineCapability;
use rocketcon_db::repositories::operational_state as operational_state_repository;
use rocketcon_db::repositories::vehicle_physical_state;
use rocketcon_db::repositories::vehicle_repository;
use rocketcon_db::repositories::propellant_repository;
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
    main_engines: Vec<MainEngineCapability>,
    throttle_ramp: Option<(Uuid, i8)>,
    fuel_tanks: Vec<(Uuid, String, f64)>,
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
        let components = vehicle_repository::list_components_for_vehicle(&pool, &vehicle_id).await?;
        let vehicle_state = resolve_vehicle_snapshot(
            &pool, vehicle_id, universe_epoch, state.captured_at_epoch()).await?;
        snapshot.battery_stored_j = Some(vehicle_state.total_stored_energy().value());
        snapshot.battery_capacity_j = Some(vehicle_state.total_battery_capacity().value());
        let mut fuel_tanks = Vec::new();
        for (entry, record) in &components {
            if let ComponentDetails::PropellantTank(tank) = record.details() {
                let name = propellant_repository::get_by_id(&pool, &tank.propellant_id()).await?
                    .map_or_else(|| tank.propellant_id().to_string(), |fuel| fuel.name().to_owned());
                let load = operational_state_repository::get_by_vehicle_component_id(
                    &pool, &entry.id()).await?.map_or(1.0, |state| state.load_fraction());
                let capacity = tank.max_propellant_mass().value();
                fuel_tanks.push((entry.id(), name.clone(), capacity));
                snapshot.fuel_reserves.push((name, capacity * load, capacity));
            }
        }
        let main_engines = components.iter().filter_map(|(entry, record)| match record.details() {
            ComponentDetails::Engine(spec) => Some(MainEngineCapability {
                id: entry.id(), ignition_type: spec.ignition_type(),
                min_throttle_fraction: spec.min_throttle_fraction(),
            }),
            _ => None,
        }).collect();
        for (entry, record) in &components {
            if matches!(record.details(), ComponentDetails::Engine(_)) {
                let load = operational_state_repository::get_by_vehicle_component_id(
                    &pool, &entry.id()).await?
                    .map_or(1.0, |state| state.load_fraction());
                snapshot.main_engine_loads.push((entry.id().to_string(), load));
            }
        }
        let vehicle_components = components.iter()
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
        snapshot.reference_vertical_speed_m_s = snapshot::reference_vertical_speed(
            snapshot.position_m,
            [body_position.0, body_position.1, body_position.2],
            snapshot.velocity_m_s,
            [body_velocity.0, body_velocity.1, body_velocity.2],
        );
        snapshot.reference_horizontal_speed_m_s = snapshot::reference_horizontal_speed(
            snapshot.reference_speed_m_s,
            snapshot.reference_vertical_speed_m_s,
        );
        snapshot.local_up_body = snapshot::local_up_body(
            snapshot.position_m,
            [body_position.0, body_position.1, body_position.2],
            state.orientation(),
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
            main_engines,
            throttle_ramp: None,
            fuel_tanks,
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
        self.advance_engine_ramp(dt_seconds).await?;
        let ramp_load_before_tick = self.throttle_ramp.and_then(|(id, _)| {
            self.snapshot.main_engine_loads.iter()
                .find(|(instance_id, _)| instance_id == &id.to_string())
                .map(|(_, load)| *load)
        });
        let report = advance_vehicle_simulation_in_session(
            &self.tick_session,
            self.vehicle_id,
            Duration::new(dt_seconds),
            self.universe_epoch,
            &self.control,
        )
        .await?;
        let mut next = FlightSnapshot::from_report(&report);
        next.fuel_reserves = self.fuel_tanks.iter().map(|(id, name, capacity)| {
            let stored = report.tank_stored_kg.iter().find(|(tank_id, _)| tank_id == id)
                .map_or(*capacity, |(_, mass)| *mass);
            (name.clone(), stored, *capacity)
        }).collect();
        self.events
            .extend(FlightEvent::between(&self.snapshot, &next));
        self.snapshot = next.clone();
        if let (Some((id, _)), Some(previous_load)) = (self.throttle_ramp, ramp_load_before_tick) {
            if previous_load > 0.0 && self.snapshot.main_engine_loads.iter()
                .any(|(instance_id, load)| instance_id == &id.to_string() && *load == 0.0) {
                self.throttle_ramp = None;
            }
        }
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
