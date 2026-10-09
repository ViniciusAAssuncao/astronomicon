use crate::ffi::{FlightEvent, FlightSnapshot, VisualComponent};
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_sim::{FlightEventKind, RocketconSession};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct Engine {
    runtime: Runtime,
    session: Option<RocketconSession>,
    event_cursor: usize,
}

pub fn create_engine() -> Result<Box<Engine>, String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    Ok(Box::new(Engine {
        runtime,
        session: None,
        event_cursor: 0,
    }))
}

impl Engine {
    pub fn load_save(&mut self, path: &str, vehicle_uuid: &str) -> Result<(), String> {
        let vehicle_id = vehicle_uuid
            .parse::<Uuid>()
            .map_err(|error| error.to_string())?;
        let next = self
            .runtime
            .block_on(RocketconSession::load(path, vehicle_id))
            .map_err(|error| error.to_string())?;
        if let Some(previous) = self.session.replace(next) {
            self.runtime.block_on(previous.close());
        }
        self.event_cursor = 0;
        Ok(())
    }

    pub fn set_control(&mut self, pitch: f64, yaw: f64, roll: f64) -> Result<(), String> {
        if ![pitch, yaw, roll].iter().all(|value| value.is_finite()) {
            return Err("control axes must be finite".into());
        }
        self.session
            .as_mut()
            .ok_or("no vehicle loaded")?
            .apply_control(VehicleControlInput::new().with_pitch_yaw_roll(pitch, yaw, roll));
        Ok(())
    }

    pub fn step(&mut self, dt_seconds: f64) -> Result<(), String> {
        self.runtime
            .block_on(
                self.session
                    .as_mut()
                    .ok_or("no vehicle loaded")?
                    .step(dt_seconds),
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<FlightSnapshot, String> {
        Ok(self
            .session
            .as_ref()
            .ok_or("no vehicle loaded")?
            .snapshot()
            .into())
    }

    pub fn vehicle_components(&self) -> Result<Vec<VisualComponent>, String> {
        Ok(self
            .session
            .as_ref()
            .ok_or("no vehicle loaded")?
            .vehicle_components()
            .iter()
            .map(VisualComponent::from)
            .collect())
    }

    pub fn take_events(&mut self) -> Result<Vec<FlightEvent>, String> {
        let events = self.session.as_ref().ok_or("no vehicle loaded")?.events();
        let next = events[self.event_cursor..]
            .iter()
            .map(|event| FlightEvent {
                kind: match event.kind {
                    FlightEventKind::AtmosphericEntry => 0,
                    FlightEventKind::AtmosphericExit => 1,
                    FlightEventKind::SurfaceContact => 2,
                    FlightEventKind::Liftoff => 3,
                },
                total_epoch_seconds: event.total_epoch_seconds,
            })
            .collect();
        self.event_cursor = events.len();
        Ok(next)
    }

    pub fn save(&self) -> Result<(), String> {
        self.runtime
            .block_on(self.session.as_ref().ok_or("no vehicle loaded")?.save())
            .map_err(|error| error.to_string())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some(session) = self.session.take() {
            self.runtime.block_on(session.close());
        }
    }
}
