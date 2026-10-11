use crate::error::RocketResult;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{ComponentDetails, ComponentOperationalState, ComponentRecord,
    VehicleComponentEntry, VehicleSnapshot};
use rocketcon_core::math::{EngineDemand, PropellantBudget, TankReserve, plan_propellant_budget};
use rocketcon_db::repositories::operational_state as operational_state_repository;
use rocketcon_db::repositories::engine_ignition as engine_ignition_repository;
use std::collections::HashMap;
use uuid::Uuid;

pub(super) struct PropellantTick {
    pub budget: PropellantBudget,
    tanks_before: Vec<TankReserve>,
    engines: Vec<EngineDemand>,
}

impl PropellantTick {
    pub async fn prepare(
        pool: &SqlitePool,
        components: &[(VehicleComponentEntry, ComponentRecord)],
        snapshot: &VehicleSnapshot,
        dt: Duration,
    ) -> RocketResult<Self> {
        let mut tanks_before = Vec::new();
        let mut active_tanks = Vec::new();
        let mut engines = Vec::new();
        for (entry, record) in components {
            match record.details() {
                ComponentDetails::PropellantTank(tank) => {
                    let load = operational_state_repository::get_by_vehicle_component_id(
                        pool, &entry.id()).await?.map_or(1.0, |state| state.load_fraction());
                    let reserve = TankReserve { id: entry.id(), propellant_id: tank.propellant_id(),
                        capacity_kg: tank.max_propellant_mass().value(),
                        stored_kg: tank.max_propellant_mass().value() * load };
                    tanks_before.push(reserve);
                    if snapshot.is_stage_active(entry.stage_index()) {
                        active_tanks.push(reserve);
                    }
                }
                ComponentDetails::Engine(engine) if snapshot.is_stage_active(entry.stage_index()) => {
                    let load = snapshot.engine_operational_states().get(&entry.id())
                        .map_or(1.0, |state| state.load_fraction());
                    engines.push(EngineDemand { id: entry.id(), fuel_id: engine.fuel_propellant_id(),
                        fuel_flow_kg_s: engine.fuel_mass_flow_rate_at_max_thrust().value(),
                        oxidizer: engine.oxidizer_propellant_id().zip(
                            engine.oxidizer_mass_flow_rate_at_max_thrust())
                            .map(|(id, flow)| (id, flow.value())), load });
                }
                _ => {}
            }
        }
        let budget = plan_propellant_budget(&engines, &active_tanks, dt.value());
        Ok(Self { budget, tanks_before, engines })
    }

    pub async fn persist(
        &self, pool: &SqlitePool, universe_epoch: Duration, at_epoch: Duration,
    ) -> RocketResult<()> {
        let updated: HashMap<Uuid, f64> = self.budget.tanks.iter()
            .map(|tank| (tank.id, tank.stored_kg)).collect();
        for tank in &self.tanks_before {
            let Some(&stored) = updated.get(&tank.id) else { continue };
            if (stored - tank.stored_kg).abs() <= 1e-12 { continue; }
            let state = ComponentOperationalState::new_simple(
                tank.id, (stored / tank.capacity_kg).clamp(0.0, 1.0),
                universe_epoch, at_epoch)?;
            operational_state_repository::upsert(pool, &state).await?;
        }
        for id in &self.budget.depleted_engines {
            let previous = operational_state_repository::get_by_vehicle_component_id(pool, id).await?;
            if engine_ignition_repository::count(pool, *id).await?.is_none() {
                engine_ignition_repository::record_ignition(pool, *id, 1).await?;
            }
            let state = ComponentOperationalState::new(
                *id, 0.0, previous.and_then(|state| state.current_gimbal_pitch()),
                previous.and_then(|state| state.current_gimbal_yaw()), universe_epoch, at_epoch)?;
            operational_state_repository::upsert(pool, &state).await?;
        }
        Ok(())
    }

    pub fn tank_stored_kg(&self) -> Vec<(Uuid, f64)> {
        let updated: HashMap<Uuid, f64> = self.budget.tanks.iter()
            .map(|tank| (tank.id, tank.stored_kg)).collect();
        self.tanks_before.iter().map(|tank|
            (tank.id, updated.get(&tank.id).copied().unwrap_or(tank.stored_kg))).collect()
    }

    pub fn main_engine_loads(&self) -> Vec<(Uuid, f64)> {
        self.engines.iter().map(|engine| (engine.id,
            if self.budget.depleted_engines.contains(&engine.id) { 0.0 } else { engine.load }))
            .collect()
    }
}
