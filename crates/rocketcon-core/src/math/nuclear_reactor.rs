use crate::physics_reference::{nuclear_fuel_properties_of, NuclearFuelType};
use astronomicon_core::units::{Duration, Energy, Luminosity, Mass};

pub fn reactor_initial_fuel_energy(fuel_type: NuclearFuelType, fuel_mass: Mass) -> Energy {
    let m = fuel_mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Energy::new(0.0);
    }

    let props = nuclear_fuel_properties_of(fuel_type);
    let spec_energy = props.realistic_specific_energy();
    spec_energy * fuel_mass
}

pub fn reactor_thermal_power_at_throttle(
    max_thermal_power: Luminosity,
    throttle_fraction: f64,
    min_throttle_fraction: Option<f64>,
) -> Luminosity {
    let p_max = max_thermal_power.value();
    if p_max <= 0.0
        || !p_max.is_finite()
        || !throttle_fraction.is_finite()
        || throttle_fraction <= 0.0
    {
        return Luminosity::new(0.0);
    }

    if let Some(min_t) = min_throttle_fraction {
        if throttle_fraction < min_t {
            return Luminosity::new(0.0);
        }
    }

    let throttle = throttle_fraction.clamp(0.0, 1.0);
    Luminosity::new(p_max * throttle)
}

pub fn reactor_electrical_power(
    thermal_power: Luminosity,
    conversion_efficiency: f64,
) -> Luminosity {
    let p_th = thermal_power.value();
    let eff = conversion_efficiency.clamp(0.0, 1.0);

    if p_th <= 0.0 || !p_th.is_finite() || !eff.is_finite() || eff <= 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(p_th * eff)
    }
}

pub fn reactor_waste_heat(
    thermal_power: Luminosity,
    electrical_power: Luminosity,
) -> Luminosity {
    let p_th = thermal_power.value();
    let p_el = electrical_power.value();

    if p_th <= 0.0 || !p_th.is_finite() {
        return Luminosity::new(0.0);
    }

    let diff = (p_th - p_el).max(0.0);
    if !diff.is_finite() {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(diff)
    }
}

pub fn reactor_fuel_energy_consumed(
    thermal_power: Luminosity,
    duration: Duration,
) -> Energy {
    let p = thermal_power.value();
    let dt = duration.value();

    if p <= 0.0 || dt <= 0.0 || !p.is_finite() || !dt.is_finite() {
        Energy::new(0.0)
    } else {
        thermal_power * duration
    }
}