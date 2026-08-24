use crate::units::{Acceleration, Density, Length, Pressure};

pub fn atmospheric_column_heat_capacity(
    surface_pressure: Pressure,
    surface_gravity: Acceleration,
    specific_heat_capacity: f64,
) -> f64 {
    let p = surface_pressure.value();
    let g = surface_gravity.value();
    let cp = specific_heat_capacity;

    if p <= 0.0 || g <= 0.0 || cp <= 0.0 || !p.is_finite() || !g.is_finite() || !cp.is_finite() {
        return 0.0;
    }

    (p / g) * cp
}

pub fn oceanic_column_heat_capacity(
    liquid_depth: Length,
    liquid_density: Density,
    specific_heat_capacity: f64,
) -> f64 {
    let d = liquid_depth.value();
    let rho = liquid_density.value();
    let cp = specific_heat_capacity;

    if d <= 0.0 || rho <= 0.0 || cp <= 0.0 || !d.is_finite() || !rho.is_finite() || !cp.is_finite()
    {
        return 0.0;
    }

    d * rho * cp
}

pub fn combined_column_heat_capacity(
    atmospheric_column_heat_capacity: f64,
    oceanic_column_heat_capacity: f64,
    ocean_coverage_fraction: f64,
) -> f64 {
    let c_atm =
        if atmospheric_column_heat_capacity.is_finite() && atmospheric_column_heat_capacity > 0.0 {
            atmospheric_column_heat_capacity
        } else {
            0.0
        };
    let c_oce = if oceanic_column_heat_capacity.is_finite() && oceanic_column_heat_capacity > 0.0 {
        oceanic_column_heat_capacity
    } else {
        0.0
    };
    let cov = ocean_coverage_fraction.clamp(0.0, 1.0);

    c_atm + cov * c_oce
}
