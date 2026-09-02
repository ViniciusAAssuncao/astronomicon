use crate::error::RocketResult;
use crate::power::consumption::resolve_component_consumption;
use crate::power::generation::resolve_component_generation;
use astronomicon_core::units::{Duration, Luminosity, Position, Quaternion};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{ComponentDetails, VehicleComponentEntry, VehicleSnapshot};
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::power_budget::{
    aggregate_power_budget, ComponentPowerContribution, VehiclePowerBudget,
};
use rocketcon_db::repositories::{
    operational_state_repository, vehicle as vehicle_repository,
};
use uuid::Uuid;

pub async fn resolve_vehicle_power_budget(
    pool: &SqlitePool,
    vehicle_snapshot: &VehicleSnapshot,
    environment: &EnvironmentSnapshot,
    vehicle_position: Position,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehiclePowerBudget> {
    let components =
        vehicle_repository::list_components_for_vehicle(pool, &vehicle_snapshot.vehicle_id())
            .await?;

    let vehicle_orientation = vehicle_snapshot
        .physical_state()
        .map(|s| s.orientation())
        .unwrap_or_else(Quaternion::identity);

    let mut contributions: Vec<(VehicleComponentEntry, ComponentPowerContribution)> =
        Vec::with_capacity(components.len());
    let mut total_generation = 0.0;
    let mut total_consumption = 0.0;

    for (entry, record) in &components {
        let is_active = vehicle_snapshot.is_stage_active(entry.stage_index());

        let gen_contribution = if is_active {
            resolve_component_generation(
                pool,
                entry,
                record,
                environment,
                vehicle_position,
                vehicle_orientation,
                universe_epoch,
                at_epoch,
            )
            .await?
        } else {
            ComponentPowerContribution::new(
                Luminosity::new(0.0),
                Luminosity::new(0.0),
                Luminosity::new(0.0),
            )
        };

        let op_state = if is_active {
            operational_state_repository::get_by_vehicle_component_id(pool, &entry.id()).await?
        } else {
            None
        };
        let con_contribution = resolve_component_consumption(entry, record, op_state);

        if is_active {
            total_generation += gen_contribution.electrical_generation.value();
            total_consumption += con_contribution.electrical_consumption.value();
        }

        contributions.push((
            entry.clone(),
            ComponentPowerContribution::new(
                gen_contribution.electrical_generation,
                con_contribution.electrical_consumption,
                gen_contribution.waste_heat + con_contribution.waste_heat,
            ),
        ));
    }

    let stored_energy = vehicle_snapshot.total_stored_energy();
    let battery_capacity = vehicle_snapshot.total_battery_capacity();

    let net_power_val = total_generation - total_consumption;

    let dumped_power = if net_power_val > 0.0 {
        if stored_energy.value() >= battery_capacity.value() || battery_capacity.value() <= 0.0 {
            Luminosity::new(net_power_val)
        } else {
            let mut total_max_charge = 0.0;
            for (entry, record) in &components {
                if !vehicle_snapshot.is_stage_active(entry.stage_index()) {
                    continue;
                }
                if let ComponentDetails::Battery(spec) = record.details() {
                    if let Some(cp) = spec.max_charge_power() {
                        total_max_charge += cp.value();
                    }
                }
            }

            if total_max_charge > 0.0 && net_power_val > total_max_charge {
                Luminosity::new(net_power_val - total_max_charge)
            } else if total_max_charge <= 0.0 {
                Luminosity::new(net_power_val)
            } else {
                Luminosity::new(0.0)
            }
        }
    } else {
        Luminosity::new(0.0)
    };

    let budget = aggregate_power_budget(
        &contributions,
        vehicle_snapshot.active_stages(),
        battery_capacity,
        stored_energy,
        dumped_power,
    );

    Ok(budget)
}

pub async fn resolve_vehicle_power_budget_for_vehicle(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    active_stages: Option<&[u32]>,
    environment: &EnvironmentSnapshot,
    vehicle_position: Position,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehiclePowerBudget> {
    let snapshot = crate::aeroespacial::resolve_vehicle_snapshot_with_options(
        pool,
        vehicle_id,
        active_stages,
        1.0,
        universe_epoch,
        at_epoch,
    )
    .await?;

    resolve_vehicle_power_budget(
        pool,
        &snapshot,
        environment,
        vehicle_position,
        universe_epoch,
        at_epoch,
    )
    .await
}