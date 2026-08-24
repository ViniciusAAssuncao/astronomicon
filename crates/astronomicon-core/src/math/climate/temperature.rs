use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{Irradiance, Length, Temperature, TemperatureGradient};

pub fn local_radiative_equilibrium_temperature(
    local_insolation: Irradiance,
    bond_albedo: f64,
) -> Temperature {
    if local_insolation.value() <= 0.0 {
        return Temperature::new(0.0);
    }
    let absorbed = (1.0 - bond_albedo.clamp(0.0, 1.0)) * local_insolation.value();
    let t4 = absorbed / STEFAN_BOLTZMANN_CONSTANT;
    Temperature::new(t4.max(0.0).powf(0.25))
}

pub fn blended_local_temperature(
    global_mean: Temperature,
    local_equilibrium: Temperature,
    thermal_inertia: f64,
) -> Temperature {
    let ti = thermal_inertia.clamp(0.0, 1.0);
    Temperature::new(ti * global_mean.value() + (1.0 - ti) * local_equilibrium.value())
}

pub fn advective_local_temperature(
    global_mean: Temperature,
    local_equilibrium: Temperature,
    redistribution_efficiency: f64,
) -> Temperature {
    let eta = redistribution_efficiency.clamp(0.0, 1.0);
    Temperature::new(eta * global_mean.value() + (1.0 - eta) * local_equilibrium.value())
}

pub fn temperature_at_altitude(
    surface_temperature: Temperature,
    altitude: Length,
    lapse_rate: TemperatureGradient,
) -> Temperature {
    let t = surface_temperature.value() - lapse_rate.value() * altitude.value();
    Temperature::new(t.max(0.0))
}
