use crate::chemistry::optics::GasOpticalProperties;
use crate::units::constants::{
    BOLTZMANN_CONSTANT, STANDARD_ATMOSPHERE_PRESSURE, STP_TEMPERATURE,
};
use crate::units::{Pressure, Temperature, Wavelength};
use std::f64::consts::PI;

pub fn rayleigh_scattering_cross_section(
    wavelength: Wavelength,
    refractivity_stp: f64,
    king_factor: f64,
) -> f64 {
    let lambda = wavelength.value();
    let delta = refractivity_stp;
    let f_k = if king_factor.is_finite() && king_factor > 0.0 {
        king_factor
    } else {
        1.0
    };

    if lambda <= 0.0 || delta <= 0.0 || !lambda.is_finite() || !delta.is_finite() {
        return 0.0;
    }

    let n = 1.0 + delta;
    let n2 = n * n;
    let n2_minus_1 = n2 - 1.0;

    let n_stp = STANDARD_ATMOSPHERE_PRESSURE / (BOLTZMANN_CONSTANT * STP_TEMPERATURE);
    let num = 8.0 * PI.powi(3) * n2_minus_1.powi(2) * f_k;
    let den = 3.0 * n_stp.powi(2) * lambda.powi(4);

    if den <= 0.0 || !den.is_finite() {
        return 0.0;
    }

    let sigma = num / den;
    if !sigma.is_finite() || sigma <= 0.0 {
        0.0
    } else {
        sigma
    }
}

pub fn molecular_number_density(pressure: Pressure, temperature: Temperature) -> f64 {
    let p = pressure.value();
    let t = temperature.value();

    if p <= 0.0 || t <= 0.0 || !p.is_finite() || !t.is_finite() {
        return 0.0;
    }

    let n = p / (BOLTZMANN_CONSTANT * t);
    if !n.is_finite() || n <= 0.0 {
        0.0
    } else {
        n
    }
}

pub fn rayleigh_scattering_coefficient(
    wavelength: Wavelength,
    refractivity_stp: f64,
    king_factor: f64,
    pressure: Pressure,
    temperature: Temperature,
) -> f64 {
    let sigma = rayleigh_scattering_cross_section(wavelength, refractivity_stp, king_factor);
    let n = molecular_number_density(pressure, temperature);
    let beta = sigma * n;

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn absorption_coefficient(
    gas_optical_properties: &GasOpticalProperties,
    wavelength: Wavelength,
    pressure: Pressure,
    temperature: Temperature,
) -> f64 {
    let sigma = gas_optical_properties.absorption_cross_section_at(wavelength);
    let n = molecular_number_density(pressure, temperature);
    let beta = sigma * n;

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}
