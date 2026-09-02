use crate::error::{ RocketError, RocketResult };
use astronomicon_core::units::{ Duration, Mass };
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentDetails,
    ComponentKind,
    ComponentRecord,
    VehicleComponentEntry,
    VehicleSnapshot,
};
use rocketcon_core::math::{ aggregate_vehicle_assembly, VehicleAssemblyTotals };
use rocketcon_db::repositories::{
    energy_reservoir as energy_reservoir_repository,
    operational_state as operational_state_repository,
    payload_state as payload_state_repository,
    vehicle as vehicle_repository,
};
use std::collections::{ HashMap, HashSet };
use uuid::Uuid;

pub async fn resolve_payload_masses(
    pool: &SqlitePool,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<HashMap<Uuid, Mass>> {
    let mut visited = HashSet::new();
    resolve_payload_masses_recursive(pool, components, universe_epoch, at_epoch, &mut visited).await
}

async fn resolve_payload_masses_recursive(
    pool: &SqlitePool,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    universe_epoch: Duration,
    at_epoch: Duration,
    visited: &mut HashSet<Uuid>
) -> RocketResult<HashMap<Uuid, Mass>> {
    let mut payload_masses = HashMap::new();

    for (entry, record) in components {
        let is_payload_dispenser_or_fairing = matches!(
            record.component().kind(),
            ComponentKind::PayloadDispenser | ComponentKind::PayloadFairing
        );

        if !is_payload_dispenser_or_fairing {
            continue;
        }

        let payload_spec = match record.details() {
            ComponentDetails::Payload(spec) => spec,
            _ => {
                continue;
            }
        };

        let is_deployed = payload_state_repository
            ::is_deployed(pool, &entry.id()).await?
            .unwrap_or(false);

        if is_deployed {
            continue;
        }

        if let Some(sub_vehicle_id) = payload_spec.contained_vehicle_id() {
            if !visited.insert(sub_vehicle_id) {
                return Err(
                    RocketError::Generic(
                        format!("payload cycle detected for vehicle '{}'", sub_vehicle_id)
                    )
                );
            }

            let sub_mass = Box::pin(
                resolve_sub_vehicle_real_mass(
                    pool,
                    &sub_vehicle_id,
                    universe_epoch,
                    at_epoch,
                    visited
                )
            ).await?;

            visited.remove(&sub_vehicle_id);

            payload_masses.insert(entry.id(), sub_mass);
            payload_masses.insert(entry.component_id(), sub_mass);
        } else if let Some(cargo_mass) = payload_spec.generic_cargo_mass() {
            payload_masses.insert(entry.id(), cargo_mass);
            payload_masses.insert(entry.component_id(), cargo_mass);
        }
    }

    Ok(payload_masses)
}

async fn resolve_sub_vehicle_real_mass(
    pool: &SqlitePool,
    sub_vehicle_id: &Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    visited: &mut HashSet<Uuid>
) -> RocketResult<Mass> {
    let sub_components = vehicle_repository::list_components_for_vehicle(
        pool,
        sub_vehicle_id
    ).await?;
    if sub_components.is_empty() {
        return Ok(Mass::new(0.0));
    }

    let sub_payload_masses = Box::pin(
        resolve_payload_masses_recursive(pool, &sub_components, universe_epoch, at_epoch, visited)
    ).await?;

    let mut total_mass_val = 0.0;

    for (sub_entry, sub_record) in &sub_components {
        total_mass_val += sub_record.component().dry_mass().value();

        if let ComponentDetails::PropellantTank(tank) = sub_record.details() {
            let op_state = operational_state_repository::get_by_vehicle_component_id(
                pool,
                &sub_entry.id()
            ).await?;
            let load_frac = match op_state {
                Some(state) => state.load_fraction().clamp(0.0, 1.0),
                None => 1.0,
            };
            total_mass_val += tank.max_propellant_mass().value() * load_frac;
        }

        if let Some(p_mass) = sub_payload_masses.get(&sub_entry.id()) {
            total_mass_val += p_mass.value();
        }
    }

    Ok(Mass::new(total_mass_val))
}

pub async fn resolve_vehicle_assembly(
    pool: &SqlitePool,
    vehicle_id: Uuid
) -> RocketResult<VehicleAssemblyTotals> {
    resolve_vehicle_assembly_at_epoch(
        pool,
        vehicle_id,
        Duration::new(0.0),
        Duration::new(0.0)
    ).await
}

pub async fn resolve_vehicle_assembly_at_epoch(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<VehicleAssemblyTotals> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let mut stages: Vec<u32> = components
        .iter()
        .map(|(entry, _)| entry.stage_index())
        .collect();
    stages.sort_unstable();
    stages.dedup();
    if stages.is_empty() {
        stages.push(0);
    }
    let payload_masses = resolve_payload_masses(pool, &components, universe_epoch, at_epoch).await?;
    let totals = aggregate_vehicle_assembly(&components, &stages, &payload_masses);
    Ok(totals)
}

pub async fn resolve_vehicle_assembly_for_stages(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: &[u32]
) -> RocketResult<VehicleAssemblyTotals> {
    resolve_vehicle_assembly_for_stages_at_epoch(
        pool,
        vehicle_id,
        active_stages,
        Duration::new(0.0),
        Duration::new(0.0)
    ).await
}

pub async fn resolve_vehicle_assembly_for_stages_at_epoch(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: &[u32],
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<VehicleAssemblyTotals> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let payload_masses = resolve_payload_masses(pool, &components, universe_epoch, at_epoch).await?;
    let totals = aggregate_vehicle_assembly(&components, active_stages, &payload_masses);
    Ok(totals)
}

pub async fn resolve_vehicle_snapshot(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<VehicleSnapshot> {
    resolve_vehicle_snapshot_with_options(
        pool,
        vehicle_id,
        None,
        1.0,
        universe_epoch,
        at_epoch
    ).await
}

pub async fn resolve_vehicle_snapshot_with_stages(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: &[u32],
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<VehicleSnapshot> {
    resolve_vehicle_snapshot_with_options(
        pool,
        vehicle_id,
        Some(active_stages),
        1.0,
        universe_epoch,
        at_epoch
    ).await
}

pub async fn resolve_vehicle_snapshot_with_options(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: Option<&[u32]>,
    propellant_load_fraction: f64,
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<VehicleSnapshot> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;

    let resolved_stages = match active_stages {
        Some(stages) => {
            if stages.is_empty() { vec![0] } else { stages.to_vec() }
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
        if
            let Some(res) = energy_reservoir_repository::get_by_vehicle_component_id(
                pool,
                &entry.id()
            ).await?
        {
            reservoir_states.insert(entry.id(), res);
        }
        if
            let Some(op) = operational_state_repository::get_by_vehicle_component_id(
                pool,
                &entry.id()
            ).await?
        {
            operational_states.insert(entry.id(), op);
        }
    }

    let payload_masses = resolve_payload_masses(pool, &components, universe_epoch, at_epoch).await?;

    let snapshot = VehicleSnapshot::from_components(
        vehicle_id,
        &components,
        resolved_stages,
        &reservoir_states,
        &operational_states,
        propellant_load_fraction,
        &payload_masses,
        universe_epoch,
        at_epoch
    )?;

    Ok(snapshot)
}

pub async fn resolve_vehicle_real_mass(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> RocketResult<Mass> {
    let mut visited = HashSet::new();
    visited.insert(vehicle_id);
    resolve_sub_vehicle_real_mass(pool, &vehicle_id, universe_epoch, at_epoch, &mut visited).await
}
