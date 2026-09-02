use crate::error::RocketResult;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentOperationalState,
    EnergyReservoirState,
    ReactionWheelState,
    VehiclePhysicalState,
    VehicleSnapshot,
};
use rocketcon_db::repositories::{
    energy_reservoir as energy_reservoir_repository,
    operational_state as operational_state_repository,
    reaction_wheel_state as reaction_wheel_state_repository,
    vehicle as vehicle_repository,
    vehicle_physical_state as vehicle_physical_state_repository,
};
use uuid::Uuid;

pub async fn resolve_universe_epoch(pool: &SqlitePool) -> RocketResult<Duration> {
    let state = astronomicon_app::universe::resolve_universe_state(pool).await?;
    Ok(state.elapsed_since_j2000())
}

pub async fn persist_vehicle_physical_state(
    pool: &SqlitePool,
    state: &VehiclePhysicalState
) -> RocketResult<()> {
    vehicle_physical_state_repository::upsert(pool, state).await?;
    Ok(())
}

pub async fn persist_energy_reservoir_state(
    pool: &SqlitePool,
    state: &EnergyReservoirState
) -> RocketResult<()> {
    energy_reservoir_repository::upsert(pool, state).await?;
    Ok(())
}

pub async fn persist_energy_reservoir_states(
    pool: &SqlitePool,
    states: &[EnergyReservoirState]
) -> RocketResult<()> {
    for state in states {
        energy_reservoir_repository::upsert(pool, state).await?;
    }
    Ok(())
}

pub async fn persist_operational_state(
    pool: &SqlitePool,
    state: &ComponentOperationalState
) -> RocketResult<()> {
    operational_state_repository::upsert(pool, state).await?;
    Ok(())
}

pub async fn persist_operational_states(
    pool: &SqlitePool,
    states: &[ComponentOperationalState]
) -> RocketResult<()> {
    for state in states {
        operational_state_repository::upsert(pool, state).await?;
    }
    Ok(())
}

pub async fn persist_reaction_wheel_state(
    pool: &SqlitePool,
    state: &ReactionWheelState
) -> RocketResult<()> {
    reaction_wheel_state_repository::upsert(pool, state).await?;
    Ok(())
}

pub async fn persist_reaction_wheel_states(
    pool: &SqlitePool,
    states: &[ReactionWheelState]
) -> RocketResult<()> {
    for state in states {
        reaction_wheel_state_repository::upsert(pool, state).await?;
    }
    Ok(())
}

pub async fn persist_vehicle_checkpoint(
    pool: &SqlitePool,
    physical_state: &VehiclePhysicalState,
    reservoir_states: &[EnergyReservoirState]
) -> RocketResult<()> {
    vehicle_physical_state_repository::upsert(pool, physical_state).await?;
    for reservoir in reservoir_states {
        energy_reservoir_repository::upsert(pool, reservoir).await?;
    }
    Ok(())
}

pub async fn persist_active_vehicles_checkpoint(
    pool: &SqlitePool,
    vehicles_data: &[(VehiclePhysicalState, Vec<EnergyReservoirState>)]
) -> RocketResult<()> {
    for (phys, reservoirs) in vehicles_data {
        persist_vehicle_checkpoint(pool, phys, reservoirs).await?;
    }
    Ok(())
}

pub async fn persist_vehicle_snapshot(
    pool: &SqlitePool,
    snapshot: &VehicleSnapshot,
    reservoir_states: &[EnergyReservoirState]
) -> RocketResult<()> {
    if let Some(phys) = snapshot.physical_state() {
        vehicle_physical_state_repository::upsert(pool, phys).await?;
    }
    for reservoir in reservoir_states {
        energy_reservoir_repository::upsert(pool, reservoir).await?;
    }
    Ok(())
}

pub async fn propagate_and_persist_active_vehicles(
    pool: &SqlitePool,
    active_vehicle_ids: &[Uuid],
    physical_state_updates: &[VehiclePhysicalState],
    reservoir_state_updates: &[EnergyReservoirState]
) -> RocketResult<()> {
    for state in physical_state_updates {
        if active_vehicle_ids.contains(&state.vehicle_id()) {
            vehicle_physical_state_repository::upsert(pool, state).await?;
        }
    }
    for res in reservoir_state_updates {
        energy_reservoir_repository::upsert(pool, res).await?;
    }
    Ok(())
}

pub async fn list_physically_active_vehicle_ids(pool: &SqlitePool) -> RocketResult<Vec<Uuid>> {
    let vehicles = vehicle_repository::list_all(pool).await?;
    let mut active = Vec::new();
    for v in vehicles {
        if vehicle_physical_state_repository::get_by_vehicle_id(pool, &v.id()).await?.is_some() {
            active.push(v.id());
        }
    }
    Ok(active)
}