use crate::domain::{ComponentDetails, ComponentRecord, VehicleComponentEntry};
use astronomicon_core::units::{Energy, Force, Luminosity, Mass, MassRate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleAssemblyTotals {
    pub total_dry_mass: Mass,
    pub total_power_consumption: Luminosity,
    pub total_battery_capacity: Energy,
    pub total_battery_max_discharge_power: Luminosity,
    pub total_solar_max_power_output: Luminosity,
    pub engine_count: u32,
    pub total_max_thrust: Force,
    pub has_cpu: bool,
    pub has_battery: bool,
    pub has_rcs: bool,
    pub rcs_thruster_count: u32,
    pub total_rcs_max_thrust: Force,
    pub has_reaction_wheel: bool,
    pub reaction_wheel_count: u32,
    pub total_propellant_capacity_by_propellant: HashMap<Uuid, Mass>,
    pub total_mass_flow_rate_by_propellant: HashMap<Uuid, MassRate>,
}

pub fn aggregate_vehicle_assembly(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    payload_masses: &HashMap<Uuid, Mass>,
) -> VehicleAssemblyTotals {
    let mut total_dry_mass = Mass::new(0.0);
    let mut total_power_consumption = Luminosity::new(0.0);
    let mut total_battery_capacity = Energy::new(0.0);
    let mut total_battery_max_discharge_power = Luminosity::new(0.0);
    let mut total_solar_max_power_output = Luminosity::new(0.0);
    let mut engine_count = 0u32;
    let mut total_max_thrust = Force::new(0.0);
    let mut has_cpu = false;
    let mut has_battery = false;
    let mut has_rcs = false;
    let mut rcs_thruster_count = 0u32;
    let mut total_rcs_max_thrust = Force::new(0.0);
    let mut has_reaction_wheel = false;
    let mut reaction_wheel_count = 0u32;
    let mut total_propellant_capacity_by_propellant: HashMap<Uuid, Mass> = HashMap::new();
    let mut total_mass_flow_rate_by_propellant: HashMap<Uuid, MassRate> = HashMap::new();

    for (entry, record) in entries {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        let comp = record.component();
        let mut comp_dry = comp.dry_mass();
        if let Some(payload_m) = payload_masses
            .get(&entry.id())
            .or_else(|| payload_masses.get(&entry.component_id()))
        {
            if payload_m.value().is_finite() && payload_m.value() > 0.0 {
                comp_dry = comp_dry + *payload_m;
            }
        }
        total_dry_mass = total_dry_mass + comp_dry;
        total_power_consumption =
            total_power_consumption + Luminosity::new(comp.power_consumption_w());

        match record.details() {
            ComponentDetails::Engine(engine) => {
                engine_count += 1;
                total_max_thrust = total_max_thrust + engine.max_thrust();

                let fuel_flow = engine.fuel_mass_flow_rate_at_max_thrust();
                let current_fuel = total_mass_flow_rate_by_propellant
                    .entry(engine.fuel_propellant_id())
                    .or_insert_with(|| MassRate::new(0.0));
                *current_fuel = *current_fuel + fuel_flow;

                if let (Some(ox_id), Some(ox_flow)) = (
                    engine.oxidizer_propellant_id(),
                    engine.oxidizer_mass_flow_rate_at_max_thrust(),
                ) {
                    let current_ox = total_mass_flow_rate_by_propellant
                        .entry(ox_id)
                        .or_insert_with(|| MassRate::new(0.0));
                    *current_ox = *current_ox + ox_flow;
                }
            }
            ComponentDetails::PropellantTank(tank) => {
                let current = total_propellant_capacity_by_propellant
                    .entry(tank.propellant_id())
                    .or_insert_with(|| Mass::new(0.0));
                *current = *current + tank.max_propellant_mass();
            }
            ComponentDetails::Battery(battery) => {
                has_battery = true;
                total_battery_capacity = total_battery_capacity + battery.capacity();
                total_battery_max_discharge_power =
                    total_battery_max_discharge_power + battery.max_discharge_power();
            }
            ComponentDetails::SolarPanel(solar) => {
                total_solar_max_power_output =
                    total_solar_max_power_output + solar.max_power_output();
            }
            ComponentDetails::Cpu => {
                has_cpu = true;
            }
            ComponentDetails::ReactionControlThruster(rcs) => {
                has_rcs = true;
                rcs_thruster_count += 1;
                total_rcs_max_thrust = total_rcs_max_thrust + rcs.max_thrust();
            }
            ComponentDetails::ReactionWheel(_) => {
                has_reaction_wheel = true;
                reaction_wheel_count += 1;
            }
            _ => {}
        }
    }

    VehicleAssemblyTotals {
        total_dry_mass,
        total_power_consumption,
        total_battery_capacity,
        total_battery_max_discharge_power,
        total_solar_max_power_output,
        engine_count,
        total_max_thrust,
        has_cpu,
        has_battery,
        has_rcs,
        rcs_thruster_count,
        total_rcs_max_thrust,
        has_reaction_wheel,
        reaction_wheel_count,
        total_propellant_capacity_by_propellant,
        total_mass_flow_rate_by_propellant,
    }
}

pub fn aggregate_vehicle_assembly_without_payloads(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
) -> VehicleAssemblyTotals {
    let empty = HashMap::new();
    aggregate_vehicle_assembly(entries, active_stages, &empty)
}
