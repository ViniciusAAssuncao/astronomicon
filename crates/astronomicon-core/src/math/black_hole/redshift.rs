use crate::math::black_hole::horizon_geometry::gravitational_radius;
use crate::units::{Length, Mass, Temperature, Wavelength};

pub fn gravitational_redshift_factor(mass: Mass, radius: Length) -> f64 {
    let rg = gravitational_radius(mass).value();
    let r = radius.value();
    if r <= 2.0 * rg || rg <= 0.0 || !r.is_finite() {
        return 1.0;
    }
    let factor = (1.0 - (2.0 * rg) / r).sqrt();
    if factor <= 0.0 || !factor.is_finite() {
        1.0
    } else {
        1.0 / factor
    }
}

pub fn gravitational_redshift_between(
    mass: Mass,
    emission_radius: Length,
    observation_radius: Length,
) -> f64 {
    let rg = gravitational_radius(mass).value();
    let r_e = emission_radius.value();
    let r_o = observation_radius.value();
    if r_e <= 2.0 * rg || r_o <= 2.0 * rg || !r_e.is_finite() || !r_o.is_finite() {
        return 1.0;
    }
    let term_e = (1.0 - (2.0 * rg) / r_e).max(1e-12).sqrt();
    let term_o = (1.0 - (2.0 * rg) / r_o).max(1e-12).sqrt();
    (term_o / term_e).max(1.0)
}

pub fn gravitationally_redshifted_wavelength(
    wavelength: Wavelength,
    mass: Mass,
    emission_radius: Length,
    observation_radius: Length,
) -> Wavelength {
    let z = gravitational_redshift_between(mass, emission_radius, observation_radius);
    Wavelength::new(wavelength.value() * z)
}

pub fn gravitationally_redshifted_temperature(
    temperature: Temperature,
    mass: Mass,
    emission_radius: Length,
    observation_radius: Length,
) -> Temperature {
    let z = gravitational_redshift_between(mass, emission_radius, observation_radius);
    if z <= 0.0 {
        temperature
    } else {
        Temperature::new(temperature.value() / z)
    }
}
