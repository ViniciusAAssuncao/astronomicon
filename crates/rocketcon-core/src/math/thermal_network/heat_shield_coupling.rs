use astronomicon_core::math::thermodynamics::conduction::conductive_heat_flux;
use astronomicon_core::units::{HeatFlux, Length, Luminosity, Temperature};
use std::f64::consts::PI;

pub fn compute_heat_shield_bondline_heat_flux(
    surface_temperature: Temperature,
    bondline_temperature: Temperature,
    shield_thickness: Length,
    material_thermal_conductivity: f64,
) -> HeatFlux {
    if surface_temperature.value() <= bondline_temperature.value() {
        return HeatFlux::new(0.0);
    }
    let delta_t = Temperature::new(surface_temperature.value() - bondline_temperature.value());
    conductive_heat_flux(
        material_thermal_conductivity,
        shield_thickness,
        delta_t,
    )
}

pub fn compute_heat_shield_conductive_power_to_structure(
    surface_temperature: Temperature,
    bondline_temperature: Temperature,
    shield_thickness: Length,
    shield_diameter: Length,
    material_thermal_conductivity: f64,
) -> Luminosity {
    let flux = compute_heat_shield_bondline_heat_flux(
        surface_temperature,
        bondline_temperature,
        shield_thickness,
        material_thermal_conductivity,
    );
    let r = shield_diameter.value() * 0.5;
    let area = PI * r * r;
    Luminosity::new((flux.value() * area).max(0.0))
}
