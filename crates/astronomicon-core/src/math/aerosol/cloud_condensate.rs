use crate::math::thermodynamics::MatterState;
use crate::units::constants::STANDARD_ATMOSPHERE_PRESSURE;
use crate::units::{Density, Pressure, Temperature};

pub fn cloud_condensate_density(
    matter_state: MatterState,
    ocean_coverage_fraction: f64,
    surface_temperature: Temperature,
    surface_pressure: Pressure,
) -> Density {
    let cov = ocean_coverage_fraction.clamp(0.0, 1.0);
    let t = surface_temperature.value();
    let p = surface_pressure.value();

    if cov <= 0.0 || t <= 0.0 || p <= 0.0 || !cov.is_finite() || !t.is_finite() || !p.is_finite() {
        return Density::new(0.0);
    }

    let base_droplet_density = 2.0e-9;
    let pressure_factor = (p / STANDARD_ATMOSPHERE_PRESSURE).clamp(0.01, 100.0).sqrt();

    let rho = match matter_state {
        MatterState::Liquid => {
            let temp_factor = (t / 288.15).clamp(0.1, 2.0);
            cov * base_droplet_density * temp_factor * pressure_factor
        }
        MatterState::Solid => {
            let temp_factor = (t / 273.15).clamp(0.05, 1.0).powi(2);
            cov * 0.5 * base_droplet_density * temp_factor * pressure_factor
        }
        MatterState::Supercritical => 2.0e-8 * pressure_factor.min(5.0),
        MatterState::Vapor => 0.0,
    };

    let clamped = rho.clamp(0.0, 0.01);
    if !clamped.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(clamped)
    }
}