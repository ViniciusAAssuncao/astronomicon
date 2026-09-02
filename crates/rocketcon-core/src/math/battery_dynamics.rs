use astronomicon_core::units::{Duration, Energy, Luminosity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BatteryState {
    pub capacity: Energy,
    pub stored_energy: Energy,
    pub max_charge_power: Option<Luminosity>,
    pub max_discharge_power: Luminosity,
}

impl BatteryState {
    pub fn new(
        capacity: Energy,
        stored_energy: Energy,
        max_charge_power: Option<Luminosity>,
        max_discharge_power: Luminosity,
    ) -> Self {
        Self {
            capacity,
            stored_energy,
            max_charge_power,
            max_discharge_power,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BatteryAllocationResult {
    pub new_stored_energy: Energy,
    pub allocated_power: Luminosity,
}

impl BatteryAllocationResult {
    pub fn new(new_stored_energy: Energy, allocated_power: Luminosity) -> Self {
        Self {
            new_stored_energy,
            allocated_power,
        }
    }
}

pub fn integrate_energy(power: Luminosity, duration: Duration) -> Energy {
    power * duration
}

pub fn battery_current_discharge_capability(
    stored_energy: Energy,
    max_discharge_power: Luminosity,
    duration: Duration,
) -> Luminosity {
    let dt = duration.value();
    let stored = stored_energy.value();
    let max_p = max_discharge_power.value();

    if dt <= 0.0
        || stored <= 0.0
        || max_p <= 0.0
        || !dt.is_finite()
        || !stored.is_finite()
        || !max_p.is_finite()
    {
        return Luminosity::new(0.0);
    }

    let energy_rate = stored / dt;
    let cap = max_p.min(energy_rate).max(0.0);
    Luminosity::new(cap)
}

pub fn battery_current_charge_capability(
    stored_energy: Energy,
    capacity: Energy,
    max_charge_power: Option<Luminosity>,
    duration: Duration,
) -> Luminosity {
    let max_p = match max_charge_power {
        Some(p) => p.value(),
        None => return Luminosity::new(0.0),
    };

    let dt = duration.value();
    let stored = stored_energy.value();
    let cap = capacity.value();

    if dt <= 0.0
        || max_p <= 0.0
        || !dt.is_finite()
        || !stored.is_finite()
        || !cap.is_finite()
        || !max_p.is_finite()
    {
        return Luminosity::new(0.0);
    }

    let remaining_capacity = (cap - stored).max(0.0);
    let energy_rate = remaining_capacity / dt;
    let charge_cap = max_p.min(energy_rate).max(0.0);
    Luminosity::new(charge_cap)
}

pub fn distribute_power_across_batteries(
    batteries: &[BatteryState],
    net_power: Luminosity,
    duration: Duration,
) -> Vec<BatteryAllocationResult> {
    let dt = duration.value();
    let p_net = net_power.value();

    if batteries.is_empty() {
        return Vec::new();
    }

    if dt <= 0.0 || !dt.is_finite() || !p_net.is_finite() || p_net == 0.0 {
        return batteries
            .iter()
            .map(|b| BatteryAllocationResult::new(b.stored_energy, Luminosity::new(0.0)))
            .collect();
    }

    if p_net < 0.0 {
        let demand = -p_net;
        let capabilities: Vec<f64> = batteries
            .iter()
            .map(|b| {
                battery_current_discharge_capability(
                    b.stored_energy,
                    b.max_discharge_power,
                    duration,
                )
                .value()
            })
            .collect();

        let total_cap: f64 = capabilities.iter().sum();

        if total_cap <= 0.0 {
            return batteries
                .iter()
                .map(|b| BatteryAllocationResult::new(b.stored_energy, Luminosity::new(0.0)))
                .collect();
        }

        let actual_total_discharge = demand.min(total_cap);

        batteries
            .iter()
            .zip(capabilities.iter())
            .map(|(b, &cap_i)| {
                let p_alloc = actual_total_discharge * (cap_i / total_cap);
                let energy_discharged = p_alloc * dt;
                let new_stored = (b.stored_energy.value() - energy_discharged).max(0.0);
                BatteryAllocationResult::new(Energy::new(new_stored), Luminosity::new(p_alloc))
            })
            .collect()
    } else {
        let supply = p_net;
        let capabilities: Vec<f64> = batteries
            .iter()
            .map(|b| {
                battery_current_charge_capability(
                    b.stored_energy,
                    b.capacity,
                    b.max_charge_power,
                    duration,
                )
                .value()
            })
            .collect();

        let total_cap: f64 = capabilities.iter().sum();

        if total_cap <= 0.0 {
            return batteries
                .iter()
                .map(|b| BatteryAllocationResult::new(b.stored_energy, Luminosity::new(0.0)))
                .collect();
        }

        let actual_total_charge = supply.min(total_cap);

        batteries
            .iter()
            .zip(capabilities.iter())
            .map(|(b, &cap_i)| {
                let p_alloc = actual_total_charge * (cap_i / total_cap);
                let energy_charged = p_alloc * dt;
                let new_stored = (b.stored_energy.value() + energy_charged).min(b.capacity.value());
                BatteryAllocationResult::new(Energy::new(new_stored), Luminosity::new(p_alloc))
            })
            .collect()
    }
}

pub fn estimated_autonomy_duration(
    total_stored_energy: Energy,
    net_power: Luminosity,
) -> Option<Duration> {
    let p = net_power.value();
    if p >= 0.0 || !p.is_finite() {
        return None;
    }

    let drain_rate = -p;
    let stored = total_stored_energy.value();

    if stored <= 0.0 || !stored.is_finite() {
        return Some(Duration::new(0.0));
    }

    Some(Duration::new(stored / drain_rate))
}