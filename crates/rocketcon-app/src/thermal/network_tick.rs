use crate::error::RocketResult;
use crate::power::thermal::VehicleThermalBudget;
use astronomicon_app::climate::resolve_irradiance_at_position;
use astronomicon_core::math::eclipse::is_in_cylindrical_shadow;
use astronomicon_core::units::constants::{COSMIC_MICROWAVE_BACKGROUND_TEMPERATURE, STEFAN_BOLTZMANN_CONSTANT};
use astronomicon_core::units::{
    Duration, HeatFlux, Length, Luminosity, Position, Quaternion, Temperature,
};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentDetails, ComponentRecord, ThermalNodeState, VehicleComponentEntry,
};
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::thermal_budget::check_material_record_thermal_structural_limits;
use rocketcon_core::math::thermal_network::{
    integrate_thermal_network, node_net_radiation, node_solar_heat_gain, ThermalNetworkState,
};
use rocketcon_db::repositories::material as material_repository;
use rocketcon_db::repositories::thermal_node_state as thermal_node_state_repository;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentThermalDiagnostic {
    pub vehicle_component_id: Uuid,
    pub component_id: Uuid,
    pub temperature: Temperature,
    pub internal_heat_generation: Luminosity,
    pub aerodynamic_heat: Luminosity,
    pub solar_heat_gain: Luminosity,
    pub radiated_power: Luminosity,
    pub max_service_temperature: Temperature,
    pub is_overheating: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleThermalTickReport {
    pub budget: VehicleThermalBudget,
    pub component_diagnostics: Vec<ComponentThermalDiagnostic>,
    pub network_state: ThermalNetworkState,
}

pub async fn advance_vehicle_thermal_network(
    pool: &SqlitePool,
    _vehicle_id: Uuid,
    network: &mut ThermalNetworkState,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    _active_stages: &[u32],
    environment: &EnvironmentSnapshot,
    vehicle_position: Position,
    vehicle_orientation: Quaternion,
    local_atmospheric_temperature: Option<Temperature>,
    dt: Duration,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<VehicleThermalTickReport> {
    let env_temp = local_atmospheric_temperature
        .unwrap_or_else(|| Temperature::new(COSMIC_MICROWAVE_BACKGROUND_TEMPERATURE));

    let irradiance = resolve_irradiance_at_position(
        pool,
        &environment.star,
        universe_epoch,
        at_epoch,
        vehicle_position,
    )
    .await?;

    let planet_radius = environment
        .planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));

    let is_eclipsed = is_in_cylindrical_shadow(
        vehicle_position,
        environment.star_position,
        environment.planet_position,
        planet_radius,
    );

    let sun_dir_world = (environment.star_position.raw() - vehicle_position.raw()).normalized();
    let sun_dir_body = vehicle_orientation.inverse().rotate_vector(sun_dir_world);

    let substep_dt = Duration::new(dt.value().clamp(0.01, 1.0));
    *network = integrate_thermal_network(
        network,
        env_temp,
        irradiance,
        sun_dir_body,
        is_eclipsed,
        dt,
        substep_dt,
    );

    let new_at_epoch = at_epoch + dt;
    for node in &network.nodes {
        let state = ThermalNodeState::new(
            node.vehicle_component_id,
            node.temperature,
            universe_epoch,
            new_at_epoch,
        )?;
        thermal_node_state_repository::upsert(pool, &state).await?;
    }

    let all_materials = material_repository::list_all(pool).await?;
    let mut materials_map: HashMap<Uuid, _> = HashMap::with_capacity(all_materials.len());
    for mat in all_materials {
        materials_map.insert(mat.material().id(), mat);
    }

    let mut component_diagnostics = Vec::with_capacity(network.nodes.len());
    let mut max_node_temp = Temperature::new(0.0);
    let mut total_internal = 0.0;
    let mut total_aero = 0.0;
    let mut total_solar = 0.0;
    let mut total_radiated = 0.0;
    let mut is_vehicle_overheating = false;
    let mut min_max_service_temp: Option<Temperature> = None;

    for node in &network.nodes {
        let t = node.temperature;
        if t.value() > max_node_temp.value() {
            max_node_temp = t;
        }

        let q_gen = node.internal_heat_generation;
        let q_aero = node.external_aerodynamic_heat;
        let q_solar = node_solar_heat_gain(node, irradiance, sun_dir_body, is_eclipsed);
        let q_rad = node_net_radiation(node, env_temp);

        total_internal += q_gen.value();
        total_aero += q_aero.value();
        total_solar += q_solar.value();
        total_radiated += q_rad.value();

        let matching_entry_record = components
            .iter()
            .find(|(e, _)| e.id() == node.vehicle_component_id);

        let (segment_name, wall_thickness) = match matching_entry_record {
            Some((entry, record)) => {
                let name = entry
                    .instance_label()
                    .unwrap_or_else(|| record.component().name());
                let thickness = match record.details() {
                    ComponentDetails::Hull(h) => h.wall_thickness(),
                    ComponentDetails::HeatShield(hs) => hs.shield_thickness(),
                    _ => Length::new(0.005),
                };
                (name, thickness)
            }
            None => ("unknown", Length::new(0.005)),
        };

        let mat_rec_opt = node.material_id.and_then(|mid| materials_map.get(&mid));
        let max_service = mat_rec_opt
            .map(|m| m.material().max_service_temperature())
            .unwrap_or(Temperature::new(1200.0));

        min_max_service_temp = Some(match min_max_service_temp {
            Some(cur_min) if max_service.value() < cur_min.value() => max_service,
            Some(cur_min) => cur_min,
            None => max_service,
        });

        let node_overheating = t.value() > max_service.value();
        if node_overheating {
            is_vehicle_overheating = true;
        }

        if let Some(mat_rec) = mat_rec_opt {
            check_material_record_thermal_structural_limits(
                segment_name,
                mat_rec,
                wall_thickness,
                t,
            )?;
        }

        component_diagnostics.push(ComponentThermalDiagnostic {
            vehicle_component_id: node.vehicle_component_id,
            component_id: node.component_id,
            temperature: t,
            internal_heat_generation: q_gen,
            aerodynamic_heat: q_aero,
            solar_heat_gain: q_solar,
            radiated_power: q_rad,
            max_service_temperature: max_service,
            is_overheating: node_overheating,
        });
    }

    let effective_ga = if max_node_temp.value() > env_temp.value() {
        let t_diff = max_node_temp.value().powi(4) - env_temp.value().powi(4);
        if t_diff > 0.0 {
            total_radiated / (STEFAN_BOLTZMANN_CONSTANT * t_diff)
        } else {
            0.0
        }
    } else {
        0.0
    };

    let budget = VehicleThermalBudget::new(
        Luminosity::new(total_internal),
        Luminosity::new(total_aero),
        Luminosity::new(total_internal + total_aero + total_solar),
        HeatFlux::new(0.0),
        HeatFlux::new(0.0),
        effective_ga,
        max_node_temp,
        min_max_service_temp.unwrap_or(max_node_temp),
        is_vehicle_overheating,
    );

    Ok(VehicleThermalTickReport {
        budget,
        component_diagnostics,
        network_state: network.clone(),
    })
}
