use crate::aeroespacial::resolve_vehicle_aerodynamics;
use crate::error::RocketResult;
use crate::thermal::network_assembly::assemble_vehicle_thermal_network;
use crate::thermal::network_tick::advance_vehicle_thermal_network;
use astronomicon_core::units::{
    Duration, HeatFlux, Luminosity, Position, Quaternion, Temperature,
};
use astronomicon_db::SqlitePool;
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_db::repositories::vehicle as vehicle_repository;
use rocketcon_db::repositories::vehicle_physical_state as vehicle_physical_state_repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleThermalBudget {
    pub total_internal_heat_generation: Luminosity,
    pub aerodynamic_heat_input: Luminosity,
    pub total_heat_input: Luminosity,
    pub stagnation_heat_flux: HeatFlux,
    pub skin_friction_heat_flux: HeatFlux,
    pub effective_radiator_ga_product: f64,
    pub equilibrium_temperature: Temperature,
    pub max_allowable_temperature: Temperature,
    pub is_overheating: bool,
}

impl VehicleThermalBudget {
    pub fn new(
        total_internal_heat_generation: Luminosity,
        aerodynamic_heat_input: Luminosity,
        total_heat_input: Luminosity,
        stagnation_heat_flux: HeatFlux,
        skin_friction_heat_flux: HeatFlux,
        effective_radiator_ga_product: f64,
        equilibrium_temperature: Temperature,
        max_allowable_temperature: Temperature,
        is_overheating: bool,
    ) -> Self {
        Self {
            total_internal_heat_generation,
            aerodynamic_heat_input,
            total_heat_input,
            stagnation_heat_flux,
            skin_friction_heat_flux,
            effective_radiator_ga_product,
            equilibrium_temperature,
            max_allowable_temperature,
            is_overheating,
        }
    }

    pub fn new_simple(
        total_internal_heat_generation: Luminosity,
        effective_radiator_ga_product: f64,
        equilibrium_temperature: Temperature,
        max_allowable_temperature: Temperature,
    ) -> Self {
        let is_overheating = equilibrium_temperature.value() > max_allowable_temperature.value();
        Self {
            total_internal_heat_generation,
            aerodynamic_heat_input: Luminosity::new(0.0),
            total_heat_input: total_internal_heat_generation,
            stagnation_heat_flux: HeatFlux::new(0.0),
            skin_friction_heat_flux: HeatFlux::new(0.0),
            effective_radiator_ga_product,
            equilibrium_temperature,
            max_allowable_temperature,
            is_overheating,
        }
    }
}

pub async fn resolve_vehicle_thermal_budget(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    environment: &EnvironmentSnapshot,
    vehicle_position: Position,
    total_internal_waste_heat: Luminosity,
) -> RocketResult<VehicleThermalBudget> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;

    let mut stages: Vec<u32> = components
        .iter()
        .map(|(e, _)| e.stage_index())
        .collect();
    stages.sort_unstable();
    stages.dedup();
    if stages.is_empty() {
        stages.push(0);
    }

    let physical_state = vehicle_physical_state_repository::get_by_vehicle_id(
        pool,
        &vehicle_id,
    ).await?;

    let aero_diag = match physical_state {
        Some(ref state) => resolve_vehicle_aerodynamics(
            pool,
            state,
            environment.planet.id(),
            environment.planet_position.raw(),
            &components,
            &stages,
            universe_epoch,
            at_epoch,
        ).await?,
        None => None,
    };

    let (air_density, relative_airspeed, mach_number) = match aero_diag {
        Some(d) => (Some(d.air_density), Some(d.relative_airspeed), Some(d.mach_number)),
        None => (None, None, None),
    };

    let mut waste_heats = HashMap::new();
    let n_comps = components.len().max(1) as f64;
    let per_comp_waste = Luminosity::new(total_internal_waste_heat.value() / n_comps);
    for (entry, _) in &components {
        waste_heats.insert(entry.id(), per_comp_waste);
    }

    let mut network = assemble_vehicle_thermal_network(
        pool,
        vehicle_id,
        &components,
        &stages,
        &waste_heats,
        air_density,
        relative_airspeed,
        mach_number,
        None,
    ).await?;

    let vehicle_orientation = physical_state
        .as_ref()
        .map(|s| s.orientation())
        .unwrap_or_else(Quaternion::identity);

    let tick_report = advance_vehicle_thermal_network(
        pool,
        vehicle_id,
        &mut network,
        &components,
        &stages,
        environment,
        vehicle_position,
        vehicle_orientation,
        None,
        Duration::new(1.0),
        universe_epoch,
        at_epoch,
    ).await?;

    let (stag_flux, skin_flux) = match aero_diag {
        Some(d) => (d.stagnation_heat_flux, HeatFlux::new(0.0)),
        None => (HeatFlux::new(0.0), HeatFlux::new(0.0)),
    };

    let mut budget = tick_report.budget;
    budget.stagnation_heat_flux = stag_flux;
    budget.skin_friction_heat_flux = skin_flux;

    Ok(budget)
}