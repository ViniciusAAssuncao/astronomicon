pub use crate::math::black_hole::schwarzschild_radius;
use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{
    Density, GravitationalParameter, Irradiance, Length, Luminosity, Mass, Speed, Temperature,
};
use std::f64::consts::PI;

pub fn stellar_luminosity(radius: Length, temperature: Temperature) -> Luminosity {
    if radius.value() <= 0.0 || temperature.value() <= 0.0 {
        return Luminosity::new(0.0);
    }
    let area = 4.0 * PI * radius.value() * radius.value();
    let t4 = temperature.value().powi(4);
    Luminosity::new(area * STEFAN_BOLTZMANN_CONSTANT * t4)
}

pub fn escape_velocity(mu: GravitationalParameter, radius: Length) -> Speed {
    if radius.value() <= 0.0 || mu.value() <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new((2.0 * mu.value() / radius.value()).sqrt())
    }
}

pub fn mean_density(mass: Mass, radius: Length) -> Density {
    if radius.value() <= 0.0 || mass.value() <= 0.0 {
        Density::new(0.0)
    } else {
        let volume = (4.0 / 3.0) * PI * radius.value().powi(3);
        Density::new(mass.value() / volume)
    }
}

pub fn orbital_irradiance(luminosity: Luminosity, distance: Length) -> Irradiance {
    if distance.value() <= 0.0 || luminosity.value() <= 0.0 {
        Irradiance::new(0.0)
    } else {
        let area = 4.0 * PI * distance.value() * distance.value();
        Irradiance::new(luminosity.value() / area)
    }
}

pub fn equilibrium_temperature(
    star_temperature: Temperature,
    star_radius: Length,
    orbital_distance: Length,
    bond_albedo: f64,
) -> Temperature {
    let luminosity = stellar_luminosity(star_radius, star_temperature);
    let irradiance = orbital_irradiance(luminosity, orbital_distance);
    let absorbed = (1.0 - bond_albedo.clamp(0.0, 1.0)) * irradiance.value();
    let t4 = absorbed / (4.0 * STEFAN_BOLTZMANN_CONSTANT);
    Temperature::new(t4.max(0.0).powf(0.25))
}