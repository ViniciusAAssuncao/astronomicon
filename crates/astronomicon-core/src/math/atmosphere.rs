use crate::units::constants::UNIVERSAL_GAS_CONSTANT;
use crate::units::{Acceleration, Density, Length, MolarMass, Pressure, Temperature};

pub fn ideal_gas_density(
    pressure: Pressure,
    molar_mass: MolarMass,
    temperature: Temperature,
) -> Density {
    if temperature.value() <= 0.0 {
        return Density::new(0.0);
    }
    let rho =
        (pressure.value() * molar_mass.value()) / (UNIVERSAL_GAS_CONSTANT * temperature.value());
    Density::new(rho)
}

pub fn scale_height(
    temperature: Temperature,
    molar_mass: MolarMass,
    gravity: Acceleration,
) -> Length {
    let denom = molar_mass.value() * gravity.value();
    if denom <= 0.0 {
        return Length::new(0.0);
    }
    let h = (UNIVERSAL_GAS_CONSTANT * temperature.value()) / denom;
    Length::new(h)
}

pub fn pressure_at_altitude(
    surface_pressure: Pressure,
    altitude: Length,
    scale_height: Length,
) -> Pressure {
    if scale_height.value() <= 0.0 {
        if altitude.value() <= 0.0 {
            return surface_pressure;
        }
        return Pressure::new(0.0);
    }
    let exponent = -altitude.value() / scale_height.value();
    Pressure::new(surface_pressure.value() * exponent.exp())
}
