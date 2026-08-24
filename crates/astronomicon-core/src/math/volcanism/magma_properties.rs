use crate::units::constants::SILICATE_MELT_THERMAL_EXPANSION;
use crate::units::{Acceleration, Density, Length, Pressure, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MagmaProperties {
    pub temperature: Temperature,
    pub density: Density,
    pub dynamic_viscosity_pa_s: f64,
    pub melt_fraction: f64,
    pub dissolved_volatile_fraction: f64,
}

pub fn magma_temperature(
    extraction_temperature: Temperature,
    solidus: Temperature,
    liquidus: Temperature,
    melt_fraction: f64,
) -> Temperature {
    let t_ext = extraction_temperature.value();
    let t_sol = solidus.value();
    let t_liq = liquidus.value();
    let phi = melt_fraction.clamp(0.0, 1.0);

    if !t_ext.is_finite() || t_ext <= 0.0 {
        return solidus;
    }

    if phi <= 0.0 {
        return solidus;
    }

    let t_magma = t_sol + phi * (t_liq - t_sol);
    Temperature::new(t_magma.max(t_sol).min(t_ext.max(t_liq)))
}

pub fn magma_density(
    solid_density: Density,
    melt_fraction: f64,
    magma_temperature: Temperature,
    solidus_temperature: Temperature,
    thermal_expansion: f64,
) -> Density {
    let rho_s = solid_density.value();
    let phi = melt_fraction.clamp(0.0, 1.0);
    let t_m = magma_temperature.value();
    let t_sol = solidus_temperature.value();
    let alpha = if thermal_expansion > 0.0 && thermal_expansion.is_finite() {
        thermal_expansion
    } else {
        SILICATE_MELT_THERMAL_EXPANSION
    };

    if rho_s <= 0.0 || !rho_s.is_finite() {
        return Density::new(0.0);
    }

    let phase_expansion_factor = 1.0 - 0.1 * phi;
    let thermal_factor = 1.0 - alpha * (t_m - t_sol).max(0.0);
    let rho_m = rho_s * phase_expansion_factor * thermal_factor.max(0.5);

    Density::new(rho_m.max(0.0))
}

pub fn magma_dynamic_viscosity(
    magma_temperature: Temperature,
    silica_mass_fraction: f64,
    dissolved_water_mass_fraction: f64,
) -> f64 {
    let t = magma_temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return 1.0e15;
    }

    let s = silica_mass_fraction.clamp(0.35, 0.85);
    let w = dissolved_water_mass_fraction.clamp(0.0, 0.15);

    let a = -4.5;
    let b = 3200.0 + 9500.0 * s - 4800.0 * w.sqrt();
    let c = 180.0 + 350.0 * s - 120.0 * w.sqrt();

    let denom = (t - c).max(10.0);
    let log10_eta = a + b / denom;

    (10.0_f64).powf(log10_eta.clamp(-3.0, 15.0))
}

pub fn buoyancy_overpressure(
    crust_density: Density,
    magma_density: Density,
    surface_gravity: Acceleration,
    magma_column_height: Length,
) -> Pressure {
    let rho_c = crust_density.value();
    let rho_m = magma_density.value();
    let g = surface_gravity.value();
    let h = magma_column_height.value();

    if g <= 0.0
        || h <= 0.0
        || !rho_c.is_finite()
        || !rho_m.is_finite()
        || !g.is_finite()
        || !h.is_finite()
    {
        return Pressure::new(0.0);
    }

    let delta_rho = rho_c - rho_m;
    Pressure::new(delta_rho * g * h)
}
