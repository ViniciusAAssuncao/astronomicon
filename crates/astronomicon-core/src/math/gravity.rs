use crate::units::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{Acceleration, GravitationalParameter, Length, Mass};

pub fn gravitational_parameter(mass: Mass) -> GravitationalParameter {
    GravitationalParameter::new(GRAVITATIONAL_CONSTANT * mass.value())
}

pub fn combined_gravitational_parameter(mass_a: Mass, mass_b: Mass) -> GravitationalParameter {
    gravitational_parameter(mass_a + mass_b)
}

pub fn surface_gravity(mu: GravitationalParameter, radius: Length) -> Acceleration {
    if radius.value() <= 0.0 {
        Acceleration::new(0.0)
    } else {
        Acceleration::new(mu.value() / (radius.value() * radius.value()))
    }
}
