pub use crate::constants::{
    DEFAULT_MAX_STRUCTURAL_TEMPERATURE_K, DEFAULT_STRUCTURAL_HULL_EMISSIVITY,
};
use crate::error::{RocketDomainError, RocketDomainResult};
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

pub fn effective_ga_product_with_hull(
    radiators: &[(f64, f64)],
    hull_area_m2: f64,
    hull_emissivity: f64,
) -> f64 {
    let rad_ga = effective_ga_product(radiators);
    let hull_ga = if hull_area_m2 > 0.0
        && hull_emissivity > 0.0
        && hull_area_m2.is_finite()
        && hull_emissivity.is_finite()
    {
        hull_area_m2 * hull_emissivity
    } else {
        0.0
    };
    rad_ga + hull_ga
}

pub fn vehicle_equilibrium_temperature(
    total_heat_input: Luminosity,
    effective_ga: f64,
) -> Temperature {
    graybody_equilibrium_temperature(total_heat_input, effective_ga)
}

pub fn vehicle_equilibrium_temperature_with_aero(
    internal_waste_heat: Luminosity,
    aerodynamic_heat: Luminosity,
    effective_ga: f64,
) -> Temperature {
    let total_heat = Luminosity::new(internal_waste_heat.value() + aerodynamic_heat.value());
    vehicle_equilibrium_temperature(total_heat, effective_ga)
}

pub fn check_thermal_structural_limits(
    equilibrium_temp: Temperature,
    max_allowable_temp: Temperature,
) -> RocketDomainResult<()> {
    if equilibrium_temp.value() > max_allowable_temp.value() {
        return Err(RocketDomainError::StructuralFailure {
            reason: format!(
                "Equilibrium temperature {:.2} K exceeded structural melting limit {:.2} K",
                equilibrium_temp.value(),
                max_allowable_temp.value()
            ),
        });
    }
    Ok(())
}