use crate::chemistry::composition_mean::composition_weighted_mean_or_zero;
use crate::error::DomainResult;
use crate::units::{Density, DynamicViscosity, Temperature};

pub fn sutherland_viscosity(
    temperature: Temperature,
    mu_ref: f64,
    t_ref: Temperature,
    c_sutherland: f64,
) -> DynamicViscosity {
    let t = temperature.value();
    let t0 = t_ref.value();
    let c = c_sutherland;

    if t <= 0.0
        || t0 <= 0.0
        || mu_ref <= 0.0
        || c < 0.0
        || !t.is_finite()
        || !t0.is_finite()
        || !mu_ref.is_finite()
        || !c.is_finite()
    {
        return DynamicViscosity::new(0.0);
    }

    let t_ratio = t / t0;
    let s_ratio = (t0 + c) / (t + c);
    let mu = mu_ref * t_ratio.powf(1.5) * s_ratio;

    if !mu.is_finite() || mu <= 0.0 {
        DynamicViscosity::new(0.0)
    } else {
        DynamicViscosity::new(mu)
    }
}

pub fn dynamic_viscosity_of(formula: &str, temperature: Temperature) -> Option<DynamicViscosity> {
    let (mu_ref, t_ref, c) = match formula {
        "H2" => (8.76e-6, 293.15, 72.0),
        "He" => (1.96e-5, 293.15, 79.4),
        "N2" => (1.75e-5, 293.15, 111.0),
        "O2" => (2.04e-5, 293.15, 127.0),
        "CO2" => (1.47e-5, 293.15, 240.0),
        "CH4" => (1.10e-5, 293.15, 164.0),
        "NH3" => (9.82e-6, 293.15, 370.0),
        "SO2" => (1.26e-5, 293.15, 416.0),
        "H2O" => (1.26e-5, 373.15, 650.0),
        "Ar" => (2.23e-5, 293.15, 144.0),
        "Ne" => (3.14e-5, 293.15, 61.0),
        "Kr" => (2.50e-5, 293.15, 188.0),
        "Xe" => (2.28e-5, 293.15, 252.0),
        "CO" => (1.75e-5, 293.15, 118.0),
        "C2H6" => (9.15e-6, 293.15, 252.0),
        _ => return None,
    };

    Some(sutherland_viscosity(
        temperature,
        mu_ref,
        Temperature::new(t_ref),
        c,
    ))
}

pub fn mean_dynamic_viscosity(
    composition: &[(String, f64)],
    temperature: Temperature,
) -> DomainResult<DynamicViscosity> {
    let mean_val = composition_weighted_mean_or_zero(composition, |formula| {
        Ok(dynamic_viscosity_of(formula, temperature)
            .map(|v| v.value())
            .unwrap_or(0.0))
    })?;

    Ok(DynamicViscosity::new(mean_val))
}

pub fn kinematic_viscosity(dynamic_viscosity: DynamicViscosity, density: Density) -> f64 {
    let eta = dynamic_viscosity.value();
    let rho = density.value();

    if eta <= 0.0 || rho <= 0.0 || !eta.is_finite() || !rho.is_finite() {
        0.0
    } else {
        eta / rho
    }
}

pub fn kinematic_viscosity_of(
    formula: &str,
    temperature: Temperature,
    density: Density,
) -> Option<f64> {
    dynamic_viscosity_of(formula, temperature).map(|eta| kinematic_viscosity(eta, density))
}

pub fn mean_kinematic_viscosity(
    composition: &[(String, f64)],
    temperature: Temperature,
    density: Density,
) -> DomainResult<f64> {
    let eta = mean_dynamic_viscosity(composition, temperature)?;
    Ok(kinematic_viscosity(eta, density))
}