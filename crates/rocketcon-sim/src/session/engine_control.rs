use super::{FlightEvent, FlightEventKind, RocketconSession};
use crate::{RocketError, RocketResult};
use astronomicon_core::units::Duration;
use rocketcon_core::domain::{ComponentOperationalState, IgnitionType};
use rocketcon_db::repositories::{engine_ignition, operational_state};
use uuid::Uuid;

pub(super) struct MainEngineCapability {
    pub id: Uuid,
    pub ignition_type: IgnitionType,
    pub min_throttle_fraction: Option<f64>,
}

impl RocketconSession {
    pub async fn set_main_engine_load(&mut self, id: Uuid, load: f64) -> RocketResult<()> {
        self.throttle_ramp = None;
        self.apply_main_engine_load(id, load, true).await
    }

    pub async fn set_main_engine_ramp(&mut self, id: Uuid, direction: i8) -> RocketResult<()> {
        if !(-1..=1).contains(&direction) {
            return Err(RocketError::Generic("engine ramp direction must be -1, 0 or 1".into()));
        }
        let engine = self.main_engines.iter().find(|engine| engine.id == id)
            .ok_or_else(|| RocketError::Generic("engine is not mounted on this vehicle".into()))?;
        if direction == 0 {
            if self.throttle_ramp.is_some_and(|(active_id, _)| active_id == id) {
                self.throttle_ramp = None;
            }
            return Ok(());
        }
        let minimum = engine.min_throttle_fraction.unwrap_or(1.0);
        let current = self.snapshot.main_engine_loads.iter()
            .find(|(instance_id, _)| instance_id == &id.to_string())
            .map_or(0.0, |(_, load)| *load);
        let first = if direction > 0 {
            if current == 0.0 { minimum } else { (current + 0.01).min(1.0) }
        } else if current <= minimum {
            0.0
        } else {
            (current - 0.01).max(minimum)
        };
        if first != current {
            self.apply_main_engine_load(id, first, true).await?;
        }
        self.throttle_ramp = Some((id, direction));
        Ok(())
    }

    pub(super) async fn advance_engine_ramp(&mut self, dt_seconds: f64) -> RocketResult<()> {
        let Some((id, direction)) = self.throttle_ramp else { return Ok(()); };
        let engine = self.main_engines.iter().find(|engine| engine.id == id)
            .ok_or_else(|| RocketError::Generic("engine is not mounted on this vehicle".into()))?;
        let minimum = engine.min_throttle_fraction.unwrap_or(1.0);
        let current = self.snapshot.main_engine_loads.iter()
            .find(|(instance_id, _)| instance_id == &id.to_string())
            .map_or(0.0, |(_, load)| *load);
        let next = if direction > 0 {
            if current >= 1.0 { current } else { (current + dt_seconds).max(minimum).min(1.0) }
        } else if current <= minimum {
            0.0
        } else {
            (current - dt_seconds).max(minimum)
        };
        if next != current {
            self.apply_main_engine_load(id, next, false).await?;
        }
        Ok(())
    }

    async fn apply_main_engine_load(&mut self, id: Uuid, load: f64,
        record_throttle_event: bool) -> RocketResult<()> {
        if !load.is_finite() || !(0.0..=1.0).contains(&load) {
            return Err(RocketError::Generic("engine load must be between 0 and 1".into()));
        }
        let engine = self.main_engines.iter().find(|engine| engine.id == id)
            .ok_or_else(|| RocketError::Generic("engine is not mounted on this vehicle".into()))?;
        if load > 0.0 && engine.min_throttle_fraction.map_or(load != 1.0, |min| load < min) {
            return Err(RocketError::Generic("requested load is outside engine throttle range".into()));
        }
        let transaction = self.tick_session.begin().await?;
        let previous = operational_state::get_by_vehicle_component_id(transaction.pool(), &id).await?;
        let current_load = previous.map_or(1.0, |state| state.load_fraction());
        if current_load == load {
            transaction.rollback().await?;
            return Ok(());
        }
        let recorded_count = engine_ignition::count(transaction.pool(), id).await?;
        let ignition_count = recorded_count.unwrap_or(0).max(if current_load > 0.0 { 1 } else { 0 });
        if current_load == 0.0 && load > 0.0 &&
            engine.ignition_type == IgnitionType::SingleBurn && ignition_count > 0 {
            transaction.rollback().await?;
            return Err(RocketError::Generic("single-burn engine cannot restart".into()));
        }
        let at_epoch = Duration::new(self.snapshot.total_epoch_seconds - self.universe_epoch.value());
        let next = ComponentOperationalState::new(
            id, load, previous.and_then(|state| state.current_gimbal_pitch()),
            previous.and_then(|state| state.current_gimbal_yaw()), self.universe_epoch, at_epoch,
        )?;
        operational_state::upsert(transaction.pool(), &next).await?;
        if current_load == 0.0 && load > 0.0 {
            engine_ignition::record_ignition(transaction.pool(), id, ignition_count + 1).await?;
        } else if recorded_count.is_none() && current_load > 0.0 {
            engine_ignition::record_ignition(transaction.pool(), id, ignition_count).await?;
        }
        transaction.commit().await?;
        if let Some((_, observed)) = self.snapshot.main_engine_loads.iter_mut()
            .find(|(instance_id, _)| instance_id == &id.to_string()) {
            *observed = load;
        }
        let kind = if current_load == 0.0 {
            FlightEventKind::EngineIgnition
        } else if load == 0.0 {
            FlightEventKind::EngineCutoff
        } else {
            FlightEventKind::EngineThrottle
        };
        if kind != FlightEventKind::EngineThrottle || record_throttle_event {
            self.events.push(FlightEvent { kind, total_epoch_seconds: self.snapshot.total_epoch_seconds });
        }
        Ok(())
    }
}
