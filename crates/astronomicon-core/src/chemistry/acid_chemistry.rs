use crate::units::constants::STANDARD_ATMOSPHERE_PRESSURE;
use crate::units::{Pressure, Temperature};

pub fn henry_constant(formula: &str, temperature: Temperature) -> Option<f64> {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return None;
    }

    let (k_h_ref, d_ln_k) = match formula {
        "CO2" => (0.034, 2400.0),
        "SO2" => (1.23, 3120.0),
        "NO2" => (0.012, 2500.0),
        "H2S" => (0.10, 2100.0),
        "HCl" => (1.1, 2000.0),
        _ => return None,
    };

    let t_ref = 298.15;
    let exponent = d_ln_k * (1.0 / t - 1.0 / t_ref);
    if !exponent.is_finite() {
        return None;
    }

    let k_h = k_h_ref * exponent.exp();
    if !k_h.is_finite() || k_h <= 0.0 {
        None
    } else {
        Some(k_h)
    }
}

pub fn acid_dissociation_constants(formula: &str) -> Option<(f64, f64)> {
    match formula {
        "CO2" => Some((4.5e-7, 4.7e-11)),
        "SO2" => Some((1.4e-2, 6.3e-8)),
        "NO2" => Some((20.0, 0.0)),
        "H2S" => Some((1.0e-7, 1.0e-13)),
        "HCl" => Some((1.0e6, 0.0)),
        _ => None,
    }
}

pub fn equilibrium_ph_from_dissolved_acids(
    acidic_gas_partial_pressures: &[(&str, Pressure)],
    temperature: Temperature,
) -> f64 {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() || acidic_gas_partial_pressures.is_empty() {
        return 7.0;
    }

    let mut dominant_c = 0.0;
    let mut dominant_ka = 0.0;
    let mut max_score = 0.0;

    for &(formula, pressure) in acidic_gas_partial_pressures {
        let p_pa = pressure.value();
        if p_pa <= 0.0 || !p_pa.is_finite() {
            continue;
        }

        let p_atm = p_pa / STANDARD_ATMOSPHERE_PRESSURE;
        let k_h = match henry_constant(formula, temperature) {
            Some(k) => k,
            None => continue,
        };

        let (ka1, _) = match acid_dissociation_constants(formula) {
            Some(k) => k,
            None => continue,
        };

        let c_aq = k_h * p_atm;
        let score = c_aq * ka1;

        if score > max_score {
            max_score = score;
            dominant_c = c_aq;
            dominant_ka = ka1;
        }
    }

    if max_score <= 0.0 || dominant_c <= 0.0 || dominant_ka <= 0.0 {
        return 7.0;
    }

    let disc = dominant_ka * dominant_ka + 4.0 * dominant_ka * dominant_c;
    if disc < 0.0 || !disc.is_finite() {
        return 7.0;
    }

    let h_plus_acid = (-dominant_ka + disc.sqrt()) / 2.0;
    let h_plus_total = (h_plus_acid * h_plus_acid + 1.0e-14).sqrt();

    if h_plus_total <= 0.0 || !h_plus_total.is_finite() {
        return 7.0;
    }

    let ph = -h_plus_total.log10();
    ph.clamp(-2.0, 14.0)
}

pub fn natural_baseline_ph(co2_partial_pressure: Pressure, temperature: Temperature) -> f64 {
    equilibrium_ph_from_dissolved_acids(&[("CO2", co2_partial_pressure)], temperature)
}
