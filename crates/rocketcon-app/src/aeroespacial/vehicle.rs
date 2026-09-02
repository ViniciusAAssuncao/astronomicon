use crate::error::RocketResult;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehicleSnapshot;
use rocketcon_core::math::{aggregate_vehicle_assembly, VehicleAssemblyTotals};
use rocketcon_db::repositories::{
    energy_reservoir as energy_reservoir_repository,
    operational_state as operational_state_repository,
    vehicle as vehicle_repository,
};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn resolve_vehicle_assembly(
    pool: &SqlitePool,
    vehicle_id: Uuid,
) -> RocketResult<VehicleAssemblyTotals> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let mut stages: Vec<u32> = components.iter().map(|(entry, _)| entry.stage_index()).collect();
    stages.sort_unstable();
    stages.dedup();
    let totals = aggregate_vehicle_assembly(&components, &stages);
    Ok(totals)
}

pub async fn resolve_vehicle_assembly_for_stages(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: &[u32],
) -> RocketResult<VehicleAssemblyTotals> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let totals = aggregate_vehicle_assembly(&components, active_stages);
    Ok(totals)
}

pub async fn resolve_vehicle_snapshot(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehicleSnapshot> {
    resolve_vehicle_snapshot_with_options(pool, vehicle_id, None, 1.0, universe_epoch, at_epoch).await
}

pub async fn resolve_vehicle_snapshot_with_stages(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: &[u32],
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehicleSnapshot> {
    resolve_vehicle_snapshot_with_options(
        pool,
        vehicle_id,
        Some(active_stages),
        1.0,
        universe_epoch,
        at_epoch,
    )
    .await
}

pub async fn resolve_vehicle_snapshot_with_options(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: Option<&[u32]>,
    propellant_load_fraction: f64,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehicleSnapshot> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;

    let resolved_stages = match active_stages {
        Some(stages) => {
            if stages.is_empty() {
                vec![0]
            } else {
                stages.to_vec()
            }
        }
        None => {
            let mut stages: Vec<u32> = components
                .iter()
                .map(|(entry, _)| entry.stage_index())
                .collect();
            stages.sort_unstable();
            stages.dedup();
            if stages.is_empty() {
                vec![0]
            } else {
                stages
            }
        }
    };

    let mut reservoir_states = HashMap::new();
    let mut operational_states = HashMap::new();

    for (entry, _) in &components {
        if let Some(res) =
            energy_reservoir_repository::get_by_vehicle_component_id(pool, &entry.id()).await?
        {
            reservoir_states.insert(entry.id(), res);
        }
        if let Some(op) =
            operational_state_repository::get_by_vehicle_component_id(pool, &entry.id()).await?
        {
            operational_states.insert(entry.id(), op);
        }
    }

    let snapshot = VehicleSnapshot::from_components(
        vehicle_id,
        &components,
        resolved_stages,
        &reservoir_states,
        &operational_states,
        propellant_load_fraction,
        universe_epoch,
        at_epoch,
    )?;

    Ok(snapshot)
}
