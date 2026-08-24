use crate::units::constants::{
    BOLTZMANN_CONSTANT, PLANCK_CONSTANT, SPEED_OF_LIGHT, STEFAN_BOLTZMANN_CONSTANT,
    WIEN_DISPLACEMENT_CONSTANT,
};
use crate::units::{Temperature, Wavelength};

pub fn peak_wavelength(temperature: Temperature) -> Wavelength {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return Wavelength::new(0.0);
    }
    Wavelength::new(WIEN_DISPLACEMENT_CONSTANT / t)
}

pub fn planck_spectral_radiance(wavelength: Wavelength, temperature: Temperature) -> f64 {
    let lambda = wavelength.value();
    let t = temperature.value();

    if lambda <= 0.0 || t <= 0.0 || !lambda.is_finite() || !t.is_finite() {
        return 0.0;
    }

    let h = PLANCK_CONSTANT;
    let c = SPEED_OF_LIGHT;
    let k_b = BOLTZMANN_CONSTANT;

    let exponent = (h * c) / (lambda * k_b * t);
    if !exponent.is_finite() || exponent > 700.0 {
        return 0.0;
    }

    let exp_term = exponent.exp() - 1.0;
    if exp_term <= 0.0 || !exp_term.is_finite() {
        return 0.0;
    }

    let numerator = 2.0 * h * c * c;
    let denominator = lambda.powi(5) * exp_term;

    if denominator <= 0.0 || !denominator.is_finite() {
        return 0.0;
    }

    let radiance = numerator / denominator;
    if !radiance.is_finite() || radiance < 0.0 {
        0.0
    } else {
        radiance
    }
}

pub fn cmb_energy_density(temperature: Temperature) -> f64 {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return 0.0;
    }

    let u = (4.0 * STEFAN_BOLTZMANN_CONSTANT / SPEED_OF_LIGHT) * t.powi(4);
    if !u.is_finite() || u < 0.0 {
        0.0
    } else {
        u
    }
}
