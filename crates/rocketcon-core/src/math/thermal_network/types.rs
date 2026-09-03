use astronomicon_core::units::{
    Length, Luminosity, Mass, Temperature, ThermalCapacitance, Vector3,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalNode {
    pub vehicle_component_id: Uuid,
    pub component_id: Uuid,
    pub stage_index: u32,
    pub thermal_capacitance: ThermalCapacitance,
    pub mass: Mass,
    pub specific_heat_capacity: f64,
    pub temperature: Temperature,
    pub exposed_area_m2: f64,
    pub emissivity: f64,
    pub solar_absorptivity: f64,
    pub internal_heat_generation: Luminosity,
    pub external_aerodynamic_heat: Luminosity,
    pub is_hull_backbone: bool,
    pub material_id: Option<Uuid>,
    pub mount_offset: Vector3,
    pub length: Length,
    pub diameter: Length,
}

impl ThermalNode {
    pub fn new(
        vehicle_component_id: Uuid,
        component_id: Uuid,
        stage_index: u32,
        thermal_capacitance: ThermalCapacitance,
        mass: Mass,
        specific_heat_capacity: f64,
        temperature: Temperature,
        exposed_area_m2: f64,
        emissivity: f64,
        solar_absorptivity: f64,
        internal_heat_generation: Luminosity,
        external_aerodynamic_heat: Luminosity,
        is_hull_backbone: bool,
        material_id: Option<Uuid>,
        mount_offset: Vector3,
        length: Length,
        diameter: Length,
    ) -> Self {
        Self {
            vehicle_component_id,
            component_id,
            stage_index,
            thermal_capacitance,
            mass,
            specific_heat_capacity,
            temperature,
            exposed_area_m2,
            emissivity,
            solar_absorptivity,
            internal_heat_generation,
            external_aerodynamic_heat,
            is_hull_backbone,
            material_id,
            mount_offset,
            length,
            diameter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermalEdge {
    pub node_a: usize,
    pub node_b: usize,
    pub conductance_w_per_k: f64,
}

impl ThermalEdge {
    pub fn new(node_a: usize, node_b: usize, conductance_w_per_k: f64) -> Self {
        Self {
            node_a,
            node_b,
            conductance_w_per_k,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalNetworkState {
    pub nodes: Vec<ThermalNode>,
    pub edges: Vec<ThermalEdge>,
}

impl ThermalNetworkState {
    pub fn new(nodes: Vec<ThermalNode>, edges: Vec<ThermalEdge>) -> Self {
        Self { nodes, edges }
    }

    pub fn node_index_by_vehicle_component_id(&self, id: &Uuid) -> Option<usize> {
        self.nodes.iter().position(|n| n.vehicle_component_id == *id)
    }

    pub fn temperatures(&self) -> Vec<Temperature> {
        self.nodes.iter().map(|n| n.temperature).collect()
    }

    pub fn set_temperatures(&mut self, temps: &[Temperature]) {
        for (node, &t) in self.nodes.iter_mut().zip(temps.iter()) {
            node.temperature = t;
        }
    }

    pub fn set_internal_heat_by_vehicle_component_id(&mut self, id: &Uuid, heat: Luminosity) {
        if let Some(idx) = self.node_index_by_vehicle_component_id(id) {
            self.nodes[idx].internal_heat_generation = heat;
        }
    }

    pub fn set_aerodynamic_heat_by_vehicle_component_id(&mut self, id: &Uuid, heat: Luminosity) {
        if let Some(idx) = self.node_index_by_vehicle_component_id(id) {
            self.nodes[idx].external_aerodynamic_heat = heat;
        }
    }

    pub fn temperature_map(&self) -> HashMap<Uuid, Temperature> {
        let mut map = HashMap::with_capacity(self.nodes.len());
        for node in &self.nodes {
            map.insert(node.vehicle_component_id, node.temperature);
        }
        map
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThermalNetworkDerivative {
    pub d_temperatures: Vec<f64>,
}

impl ThermalNetworkDerivative {
    pub fn new(d_temperatures: Vec<f64>) -> Self {
        Self { d_temperatures }
    }

    pub fn zero(size: usize) -> Self {
        Self {
            d_temperatures: vec![0.0; size],
        }
    }
}