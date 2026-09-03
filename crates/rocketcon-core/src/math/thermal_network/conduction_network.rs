use super::types::{ ThermalEdge, ThermalNode, ThermalNetworkState };
use crate::domain::{
    ComponentDetails,
    ComponentKind,
    ComponentRecord,
    MaterialRecord,
    VehicleComponentEntry,
};
use astronomicon_core::units::{ Luminosity, Temperature, ThermalCapacitance };
use std::collections::HashMap;
use std::f64::consts::PI;
use uuid::Uuid;

const DEFAULT_SPECIFIC_HEAT_CAPACITY: f64 = 900.0;
const DEFAULT_THERMAL_CONDUCTIVITY: f64 = 170.0;
const DEFAULT_EMISSIVITY: f64 = 0.85;
const DEFAULT_SOLAR_ABSORPTIVITY: f64 = 0.85;
const MIN_CONDUCTION_DISTANCE_M: f64 = 0.05;
const CONTACT_AREA_FRACTION: f64 = 0.25;

pub fn build_thermal_network(
    entries_and_records: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    materials: &HashMap<Uuid, MaterialRecord>,
    component_material_overrides: &HashMap<Uuid, Uuid>,
    initial_temperatures: &HashMap<Uuid, Temperature>,
    default_temperature: Temperature
) -> ThermalNetworkState {
    let mut nodes = Vec::new();
    let mut node_conductivities = Vec::new();

    for (entry, record) in entries_and_records {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        let comp = record.component();
        let direct_material_id = match record.details() {
            ComponentDetails::Hull(spec) => Some(spec.material_id()),
            ComponentDetails::HeatShield(spec) => Some(spec.material_id()),
            _ => None,
        };

        let resolved_material_id = component_material_overrides
            .get(&entry.id())
            .or_else(|| component_material_overrides.get(&entry.component_id()))
            .copied()
            .or(direct_material_id);

        let material_rec = resolved_material_id.and_then(|mid| materials.get(&mid));
        let material = material_rec.map(|m| m.material());

        let cp = material
            .map(|m| m.specific_heat_capacity_j_per_kg_k())
            .unwrap_or(DEFAULT_SPECIFIC_HEAT_CAPACITY);

        let k = material
            .map(|m| m.thermal_conductivity_w_per_m_k())
            .unwrap_or(DEFAULT_THERMAL_CONDUCTIVITY);

        let (emissivity, solar_absorptivity) = match record.details() {
            ComponentDetails::SolarPanel(spec) =>
                (
                    material.map(|m| m.emissivity()).unwrap_or(DEFAULT_EMISSIVITY),
                    spec.effective_solar_absorptivity(),
                ),
            ComponentDetails::Radiator(spec) => (spec.emissivity(), spec.solar_absorptivity()),
            _ =>
                (
                    material.map(|m| m.emissivity()).unwrap_or(DEFAULT_EMISSIVITY),
                    material.map(|m| m.solar_absorptivity()).unwrap_or(DEFAULT_SOLAR_ABSORPTIVITY),
                ),
        };

        let exposed_area = match record.details() {
            ComponentDetails::Hull(_) => PI * comp.diameter().value() * comp.length().value(),
            ComponentDetails::HeatShield(_) => {
                let r = comp.diameter().value() * 0.5;
                PI * r * r
            }
            ComponentDetails::SolarPanel(spec) => spec.surface_area_m2(),
            ComponentDetails::Radiator(spec) => spec.radiating_area_m2(),
            ComponentDetails::Engine(_) =>
                0.5 * PI * comp.diameter().value() * comp.length().value(),
            _ => 0.0,
        };

        let mass_val = comp.dry_mass().value();
        let capacitance_val = mass_val * cp;
        let is_backbone = comp.kind() == ComponentKind::Hull;

        let temp = initial_temperatures
            .get(&entry.id())
            .or_else(|| initial_temperatures.get(&entry.component_id()))
            .copied()
            .unwrap_or(default_temperature);

        let node = ThermalNode::new(
            entry.id(),
            entry.component_id(),
            entry.stage_index(),
            ThermalCapacitance::new(capacitance_val.max(1.0)),
            comp.dry_mass(),
            cp,
            temp,
            exposed_area.max(0.0),
            emissivity.clamp(0.01, 1.0),
            solar_absorptivity.clamp(0.01, 1.0),
            Luminosity::new(0.0),
            Luminosity::new(0.0),
            is_backbone,
            resolved_material_id,
            entry.mount_offset(),
            comp.length(),
            comp.diameter()
        );

        nodes.push(node);
        node_conductivities.push(k.max(0.1));
    }

    let mut edges = Vec::new();
    let n_nodes = nodes.len();

    let mut stages_present = Vec::new();
    for node in &nodes {
        if !stages_present.contains(&node.stage_index) {
            stages_present.push(node.stage_index);
        }
    }

    let mut stage_backbone_indices = HashMap::new();
    for &stg in &stages_present {
        let mut best_backbone = None;
        let mut max_mass = f64::NEG_INFINITY;

        for (idx, node) in nodes.iter().enumerate() {
            if node.stage_index != stg {
                continue;
            }
            if node.is_hull_backbone {
                best_backbone = Some(idx);
                break;
            }
            if node.mass.value() > max_mass {
                max_mass = node.mass.value();
                best_backbone = Some(idx);
            }
        }

        if let Some(bb_idx) = best_backbone {
            stage_backbone_indices.insert(stg, bb_idx);
        }
    }

    for i in 0..n_nodes {
        let node_stg = nodes[i].stage_index;
        if let Some(&bb_idx) = stage_backbone_indices.get(&node_stg) {
            if i == bb_idx {
                continue;
            }

            let dist = (nodes[i].mount_offset - nodes[bb_idx].mount_offset)
                .magnitude()
                .max(MIN_CONDUCTION_DISTANCE_M);

            let d_eff = nodes[i].diameter.value().min(nodes[bb_idx].diameter.value());
            let r_eff = d_eff * 0.5;
            let area = PI * r_eff * r_eff * CONTACT_AREA_FRACTION;

            let k1 = node_conductivities[i];
            let k2 = node_conductivities[bb_idx];
            let k_eff = (2.0 * k1 * k2) / (k1 + k2).max(1e-6);

            let conductance = (k_eff * area) / dist;
            if conductance.is_finite() && conductance > 0.0 {
                edges.push(ThermalEdge::new(i, bb_idx, conductance));
            }
        }
    }

    stages_present.sort_unstable();
    for window in stages_present.windows(2) {
        let stg_a = window[0];
        let stg_b = window[1];

        if
            let (Some(&bb_a), Some(&bb_b)) = (
                stage_backbone_indices.get(&stg_a),
                stage_backbone_indices.get(&stg_b),
            )
        {
            let dist = (nodes[bb_a].mount_offset - nodes[bb_b].mount_offset)
                .magnitude()
                .max(MIN_CONDUCTION_DISTANCE_M);

            let d_eff = nodes[bb_a].diameter.value().min(nodes[bb_b].diameter.value());
            let r_eff = d_eff * 0.5;
            let area = PI * r_eff * r_eff * CONTACT_AREA_FRACTION;

            let k1 = node_conductivities[bb_a];
            let k2 = node_conductivities[bb_b];
            let k_eff = (2.0 * k1 * k2) / (k1 + k2).max(1e-6);

            let conductance = (k_eff * area) / dist;
            if conductance.is_finite() && conductance > 0.0 {
                edges.push(ThermalEdge::new(bb_a, bb_b, conductance));
            }
        }
    }

    ThermalNetworkState::new(nodes, edges)
}
