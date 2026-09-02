use crate::error::RocketResult;
use astronomicon_core::units::{Duration, Energy, Luminosity};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{ComponentDetails, EnergyReservoirState};
use rocketcon_core::math::battery_dynamics::{distribute_power_across_batteries, BatteryState};
use rocketcon_db::repositories::{energy_reservoir_repository, vehicle as vehicle_repository};
use uuid::Uuid;

pub async fn resolve_vehicle_battery_state(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    _universe_epoch: Duration,
    _at_epoch: Duration,
) -> RocketResult<(Energy, Energy)> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let mut total_stored = 0.0;
    let mut total_capacity = 0.0;

    for (entry, record) in components {
        if let ComponentDetails::Battery(spec) = record.details() {
            let cap = spec.capacity();
            total_capacity += cap.value();

            let reservoir =
                energy_reservoir_repository::get_by_vehicle_component_id(pool, &entry.id()).await?;
            let stored = match reservoir {
                Some(state) => state.stored_energy().value().min(cap.value()),
                None => cap.value(),
            };
            total_stored += stored;
        }
    }

    Ok((Energy::new(total_stored), Energy::new(total_capacity)))
}

pub async fn apply_power_delta(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    net_power: Luminosity,
    duration: Duration,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<Luminosity> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;

    let mut battery_entries = Vec::new();
    let mut battery_states = Vec::new();

    for (entry, record) in components {
        if let ComponentDetails::Battery(spec) = record.details() {
            let cap = spec.capacity();
            let reservoir =
                energy_reservoir_repository::get_by_vehicle_component_id(pool, &entry.id()).await?;
            let stored = match reservoir {
                Some(state) => state.stored_energy(),
                None => cap,
            };

            let state = BatteryState::new(
                cap,
                stored,
                spec.max_charge_power(),
                spec.max_discharge_power(),
            );
            battery_entries.push(entry);
            battery_states.push(state);
        }
    }

    if battery_states.is_empty() {
        if net_power.value() > 0.0 {
            return Ok(net_power);
        } else {
            return Ok(Luminosity::new(0.0));
        }
    }

    let results = distribute_power_across_batteries(&battery_states, net_power, duration);

    let mut total_allocated_power = 0.0;
    for (entry, alloc_res) in battery_entries.iter().zip(results.iter()) {
        total_allocated_power += alloc_res.allocated_power.value();
        let new_state = EnergyReservoirState::new(
            entry.id(),
            alloc_res.new_stored_energy,
            universe_epoch,
            at_epoch,
        )?;
        energy_reservoir_repository::upsert(pool, &new_state).await?;
    }

    let dumped_power = if net_power.value() > 0.0 {
        Luminosity::new((net_power.value() - total_allocated_power).max(0.0))
    } else {
        Luminosity::new(0.0)
    };

    Ok(dumped_power)
}
