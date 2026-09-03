use crate::aeroespacial::resolve_vehicle_aerodynamics;
use crate::error::RocketResult;
use astronomicon_core::units::{Duration, HeatFlux, Luminosity, Position, Temperature};
use astronomicon_db::SqlitePool;
use rocketcon_core::constants::DEFAULT_STRUCTURAL_HULL_EMISSIVITY;
use rocketcon_core::domain::ComponentDetails;
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::aerothermodynamics::{
    evaluate_vehicle_aerothermodynamics, vehicle_geometry_thermal_properties,
};
use rocketcon_core::math::thermal_budget::{
    check_material_record_thermal_structural_limits, effective_ga_product_with_hull,
    vehicle_equilibrium_temperature_with_aero,
};
use rocketcon_db::repositories::material as material_repository;
use rocketcon_db::repositories::vehicle as vehicle_repository;
use rocketcon_db::repositories::vehicle_physical_state as vehicle_physical_state_repository;
use serde::{Deserialize, Serialize};
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
    _vehicle_position: Position,
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

    let mut radiators = Vec::new();
    for (entry, record) in &components {
        if !stages.contains(&entry.stage_index()) {
            continue;
        }
        if let ComponentDetails::Radiator(spec) = record.details() {
            radiators.push((spec.radiating_area_m2(), spec.emissivity()));
        }
    }

    let physical_state = vehicle_physical_state_repository::get_by_vehicle_id(
        pool,
        &vehicle_id,
    ).await?;

    let aero_results = match physical_state {
        Some(ref state) => {
            let diag_opt = resolve_vehicle_aerodynamics(
                pool,
                state,
                environment.planet.id(),
                environment.planet_position.raw(),
                &components,
                &stages,
                universe_epoch,
                at_epoch,
            ).await?;

            diag_opt.map(|diag| {
                evaluate_vehicle_aerothermodynamics(
                    &components,
                    &stages,
                    diag.air_density,
                    diag.relative_airspeed,
                    diag.mach_number,
                )
            })
        }
        None => None,
    };

    let (aero_power, stag_flux, skin_flux, hull_area) = match aero_results {
        Some(res) => (
            res.total_aerodynamic_heat_power,
            res.stagnation_heat_flux,
            res.skin_friction_heat_flux,
            res.nose_area_m2 + res.side_area_m2,
        ),
        None => {
            let (_, nose_area, side_area) =
                vehicle_geometry_thermal_properties(&components, &stages);
            (
                Luminosity::new(0.0),
                HeatFlux::new(0.0),
                HeatFlux::new(0.0),
                nose_area + side_area,
            )
        }
    };

    let effective_ga = effective_ga_product_with_hull(
        &radiators,
        hull_area,
        DEFAULT_STRUCTURAL_HULL_EMISSIVITY,
    );
    let total_heat = Luminosity::new(total_internal_waste_heat.value() + aero_power.value());
    let eq_temp = vehicle_equilibrium_temperature_with_aero(
        total_internal_waste_heat,
        aero_power,
        effective_ga,
    );

    let mut min_allowable_temp: Option<Temperature> = None;

    for (entry, record) in &components {
        if !stages.contains(&entry.stage_index()) {
            continue;
        }

        if let ComponentDetails::Hull(hull) = record.details() {
            if let Some(material_record) =
                material_repository::get_by_id(pool, &hull.material_id()).await?
            {
                let segment_name = entry
                    .instance_label()
                    .unwrap_or_else(|| record.component().name());
                let mat_max_service = material_record.material().max_service_temperature();

                min_allowable_temp = Some(match min_allowable_temp {
                    Some(cur_min) if mat_max_service.value() < cur_min.value() => mat_max_service,
                    Some(cur_min) => cur_min,
                    None => mat_max_service,
                });

                check_material_record_thermal_structural_limits(
                    segment_name,
                    &material_record,
                    hull.wall_thickness(),
                    eq_temp,
                )?;
            }
        }
    }

    let max_temp = min_allowable_temp.unwrap_or(eq_temp);
    let is_overheating = eq_temp.value() > max_temp.value();

    Ok(VehicleThermalBudget {
        total_internal_heat_generation: total_internal_waste_heat,
        aerodynamic_heat_input: aero_power,
        total_heat_input: total_heat,
        stagnation_heat_flux: stag_flux,
        skin_friction_heat_flux: skin_flux,
        effective_radiator_ga_product: effective_ga,
        equilibrium_temperature: eq_temp,
        max_allowable_temperature: max_temp,
        is_overheating,
    })
}