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
        self.events.push(FlightEvent { kind, total_epoch_seconds: self.snapshot.total_epoch_seconds });
        Ok(())
    }
}
