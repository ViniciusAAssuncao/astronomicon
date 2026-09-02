use crate::domain::VehicleComponentEntry;
use crate::math::battery_dynamics::estimated_autonomy_duration;
use astronomicon_core::units::{Duration, Energy, Luminosity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VehiclePowerStatus {
    Nominal,
    Critical,
    Depleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComponentPowerContribution {
    pub electrical_generation: Luminosity,
    pub electrical_consumption: Luminosity,
    pub waste_heat: Luminosity,
}

impl ComponentPowerContribution {
    pub fn new(
        electrical_generation: Luminosity,
        electrical_consumption: Luminosity,
        waste_heat: Luminosity,
    ) -> Self {
        Self {
            electrical_generation,
            electrical_consumption,
            waste_heat,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehiclePowerBudget {
    pub total_generation: Luminosity,
    pub total_consumption: Luminosity,
    pub net_power: Luminosity,
    pub dumped_power: Luminosity,
    pub total_internal_waste_heat: Luminosity,
    pub total_stored_energy: Energy,
    pub total_battery_capacity: Energy,
    pub state_of_charge_fraction: f64,
    pub estimated_autonomy: Option<Duration>,
    pub status: VehiclePowerStatus,
}

impl VehiclePowerBudget {
    pub fn new(
        total_generation: Luminosity,
        total_consumption: Luminosity,
        net_power: Luminosity,
        dumped_power: Luminosity,
        total_internal_waste_heat: Luminosity,
        total_stored_energy: Energy,
        total_battery_capacity: Energy,
        state_of_charge_fraction: f64,
        estimated_autonomy: Option<Duration>,
        status: VehiclePowerStatus,
    ) -> Self {
        Self {
            total_generation,
            total_consumption,
            net_power,
            dumped_power,
            total_internal_waste_heat,
            total_stored_energy,
            total_battery_capacity,
            state_of_charge_fraction,
            estimated_autonomy,
            status,
        }
    }
}

pub fn component_power_consumption(rated_power: Luminosity, load_fraction: f64) -> Luminosity {
    let p = rated_power.value();
    if p <= 0.0 || !p.is_finite() || !load_fraction.is_finite() || load_fraction <= 0.0 {
        return Luminosity::new(0.0);
    }

    let load = load_fraction.clamp(0.0, 1.0);
    Luminosity::new(p * load)
}

pub fn component_consumption_waste_heat(consumed_power: Luminosity) -> Luminosity {
    let p = consumed_power.value();
    if p <= 0.0 || !p.is_finite() {
        return Luminosity::new(0.0);
    }

    Luminosity::new(p)
}

pub fn aggregate_power_budget(
    contributions: &[(VehicleComponentEntry, ComponentPowerContribution)],
    active_stages: &[u32],
    battery_capacity: Energy,
    battery_stored: Energy,
    dumped_power: Luminosity,
) -> VehiclePowerBudget {
    let mut total_gen = 0.0;
    let mut total_con = 0.0;
    let mut comp_waste = 0.0;

    for (entry, c) in contributions {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        if c.electrical_generation.value().is_finite() && c.electrical_generation.value() > 0.0 {
            total_gen += c.electrical_generation.value();
        }
        if c.electrical_consumption.value().is_finite() && c.electrical_consumption.value() > 0.0 {
            total_con += c.electrical_consumption.value();
        }
        if c.waste_heat.value().is_finite() && c.waste_heat.value() > 0.0 {
            comp_waste += c.waste_heat.value();
        }
    }

    let net_power_val = total_gen - total_con;
    let net_power = Luminosity::new(net_power_val);

    let dumped_val = if dumped_power.value().is_finite() && dumped_power.value() > 0.0 {
        dumped_power.value()
    } else {
        0.0
    };
    let dumped = Luminosity::new(dumped_val);

    let total_internal_waste_heat = Luminosity::new(comp_waste + dumped_val);

    let cap_val = battery_capacity.value();
    let stored_val = battery_stored.value();

    let state_of_charge_fraction =
        if cap_val > 0.0 && cap_val.is_finite() && stored_val.is_finite() {
            (stored_val / cap_val).clamp(0.0, 1.0)
        } else {
            0.0
        };

    let status = if stored_val <= 0.0 || !stored_val.is_finite() {
        VehiclePowerStatus::Depleted
    } else if state_of_charge_fraction < 0.10 {
        VehiclePowerStatus::Critical
    } else {
        VehiclePowerStatus::Nominal
    };

    let estimated_autonomy = estimated_autonomy_duration(battery_stored, net_power);

    VehiclePowerBudget {
        total_generation: Luminosity::new(total_gen),
        total_consumption: Luminosity::new(total_con),
        net_power,
        dumped_power: dumped,
        total_internal_waste_heat,
        total_stored_energy: battery_stored,
        total_battery_capacity: battery_capacity,
        state_of_charge_fraction,
        estimated_autonomy,
        status,
    }
}