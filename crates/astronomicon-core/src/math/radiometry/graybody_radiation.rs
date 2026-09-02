use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{Luminosity, Temperature};

pub fn graybody_radiated_power(
    emissivity: f64,
    area_m2: f64,
    temperature: Temperature,
) -> Luminosity {
    let eps = emissivity.clamp(0.0, 1.0);
    let a = area_m2;
    let t = temperature.value();

    if eps <= 0.0 || a <= 0.0 || t <= 0.0 || !eps.is_finite() || !a.is_finite() || !t.is_finite() {
        return Luminosity::new(0.0);
    }

    let power = eps * STEFAN_BOLTZMANN_CONSTANT * a * t.powi(4);
    if !power.is_finite() || power < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(power)
    }
}

pub fn net_graybody_radiated_power(
    emissivity: f64,
    area_m2: f64,
    temperature: Temperature,
    environment_temperature: Temperature,
) -> Luminosity {
    let eps = emissivity.clamp(0.0, 1.0);
    let a = area_m2;
    let t = temperature.value().max(0.0);
    let t_env = environment_temperature.value().max(0.0);

    if eps <= 0.0
        || a <= 0.0
        || !eps.is_finite()
        || !a.is_finite()
        || !t.is_finite()
        || !t_env.is_finite()
    {
        return Luminosity::new(0.0);
    }

    let t4_diff = t.powi(4) - t_env.powi(4);
    let power = eps * STEFAN_BOLTZMANN_CONSTANT * a * t4_diff;
    if !power.is_finite() {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(power)
    }
}

pub fn graybody_equilibrium_temperature(
    net_heat_input: Luminosity,
    effective_ga_product: f64,
) -> Temperature {
    let q = net_heat_input.value();
    let ga = effective_ga_product;

    if q <= 0.0 || ga <= 0.0 || !q.is_finite() || !ga.is_finite() {
        return Temperature::new(0.0);
    }

    let denom = STEFAN_BOLTZMANN_CONSTANT * ga;
    if denom <= 0.0 || !denom.is_finite() {
        return Temperature::new(0.0);
    }

    let t4 = q / denom;
    if !t4.is_finite() || t4 <= 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(t4.powf(0.25))
    }
}