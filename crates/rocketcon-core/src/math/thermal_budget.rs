use astronomicon_core::math::graybody_equilibrium_temperature;
use astronomicon_core::units::{Luminosity, Temperature};

pub fn effective_ga_product(radiators: &[(f64, f64)]) -> f64 {
    radiators
        .iter()
        .filter_map(|&(area, emissivity)| {
            if area > 0.0 && emissivity > 0.0 && area.is_finite() && emissivity.is_finite() {
                Some(area * emissivity)
            } else {
                None
            }
        })
        .sum()
}

pub fn vehicle_equilibrium_temperature(
    total_internal_heat: Luminosity,
    effective_ga: f64,
) -> Temperature {
    graybody_equilibrium_temperature(total_internal_heat, effective_ga)
}