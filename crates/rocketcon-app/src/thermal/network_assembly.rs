use crate::error::RocketResult;
use astronomicon_core::units::{Density, Luminosity, Speed, Temperature};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{
    ComponentDetails, ComponentRecord, MaterialRecord, VehicleComponentEntry,
};
use rocketcon_core::math::aerothermodynamics::evaluate_vehicle_aerothermodynamics;
use rocketcon_core::math::thermal_network::{ThermalNetworkState, build_thermal_network};
use rocketcon_db::repositories::component_attributes::{fetch_attribute_map, optional_uuid};
use rocketcon_db::repositories::material as material_repository;
use rocketcon_db::repositories::thermal_node_state as thermal_node_state_repository;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn assemble_vehicle_thermal_network(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    component_waste_heats: &HashMap<Uuid, Luminosity>,
    air_density: Option<Density>,
    relative_airspeed: Option<Speed>,
    mach_number: Option<f64>,
    default_temperature: Option<Temperature>,
) -> RocketResult<ThermalNetworkState> {
    let (network, _) = assemble_vehicle_thermal_network_with_materials(
        pool,
        vehicle_id,
        components,
        active_stages,
        component_waste_heats,
        air_density,
        relative_airspeed,
        mach_number,
        default_temperature,
    )
    .await?;
    Ok(network)
}

pub(crate) async fn assemble_vehicle_thermal_network_with_materials(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    component_waste_heats: &HashMap<Uuid, Luminosity>,
    air_density: Option<Density>,
    relative_airspeed: Option<Speed>,
    mach_number: Option<f64>,
    default_temperature: Option<Temperature>,
) -> RocketResult<(ThermalNetworkState, HashMap<Uuid, MaterialRecord>)> {
    let all_materials = material_repository::list_all(pool).await?;
    let mut materials_map: HashMap<Uuid, MaterialRecord> =
        HashMap::with_capacity(all_materials.len());
    for mat in all_materials {
        materials_map.insert(mat.material().id(), mat);
    }

    let mut component_material_overrides = HashMap::new();
    let mut material_ids_by_component = HashMap::<Uuid, Option<Uuid>>::new();
    for (entry, record) in components {
        let component_id = entry.component_id();
        let material_id = if let Some(&cached) = material_ids_by_component.get(&component_id) {
            cached
        } else {
            match fetch_attribute_map(pool, &component_id).await {
                Ok(attributes) => {
                    let resolved = optional_uuid(&attributes, &component_id, "material_id")
                        .ok()
                        .flatten();
                    material_ids_by_component.insert(component_id, resolved);
                    resolved
                }
                Err(_) => None,
            }
        };
        if let Some(mat_id) = material_id {
            component_material_overrides.insert(entry.id(), mat_id);
            component_material_overrides.insert(component_id, mat_id);
        }
        if let ComponentDetails::Hull(hull) = record.details() {
            component_material_overrides.insert(entry.id(), hull.material_id());
        }
        if let ComponentDetails::HeatShield(shield) = record.details() {
            component_material_overrides.insert(entry.id(), shield.material_id());
        }
    }

    let existing_states =
        thermal_node_state_repository::list_for_vehicle(pool, &vehicle_id).await?;
    let mut initial_temperatures = HashMap::with_capacity(existing_states.len());
    for st in existing_states {
        initial_temperatures.insert(st.vehicle_component_id(), st.current_temperature());
    }

    let def_temp = default_temperature.unwrap_or_else(|| Temperature::new(293.15));

    let mut network = build_thermal_network(
        components,
        active_stages,
        &materials_map,
        &component_material_overrides,
        &initial_temperatures,
        def_temp,
    );

    for (v_comp_id, &waste_heat) in component_waste_heats {
        network.set_internal_heat_by_vehicle_component_id(v_comp_id, waste_heat);
    }

    if let (Some(rho), Some(v_rel), Some(mach)) = (air_density, relative_airspeed, mach_number) {
        let aero_res =
            evaluate_vehicle_aerothermodynamics(components, active_stages, rho, v_rel, mach);

        let mut foremost_idx: Option<usize> = None;
        let mut foremost_z = f64::NEG_INFINITY;

        for (idx, node) in network.nodes.iter().enumerate() {
            let z_front = node.mount_offset.2 + node.length.value() * 0.5;
            if z_front > foremost_z {
                foremost_z = z_front;
                foremost_idx = Some(idx);
            }
        }

        let stag_power =
            Luminosity::new(aero_res.stagnation_heat_flux.value() * aero_res.nose_area_m2);
        if let Some(nose_idx) = foremost_idx {
            network.nodes[nose_idx].external_aerodynamic_heat =
                network.nodes[nose_idx].external_aerodynamic_heat + stag_power;
        }

        let skin_flux = aero_res.skin_friction_heat_flux.value();
        if skin_flux > 0.0 {
            for node in &mut network.nodes {
                if node.exposed_area_m2 > 0.0 {
                    let node_skin_power = Luminosity::new(skin_flux * node.exposed_area_m2);
                    node.external_aerodynamic_heat =
                        node.external_aerodynamic_heat + node_skin_power;
                }
            }
        }
    }

    Ok((network, materials_map))
}
