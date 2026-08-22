use crate::domain::PlanetKind;
use crate::units::constants::{
    CARBON_CRUST_THERMAL_CONDUCTIVITY, CARBON_SOLIDUS_BASE_K, EARTH_MASS,
    ICE_CRUST_THERMAL_CONDUCTIVITY, ICE_SOLIDUS_BASE_K, SILICATE_CRUST_THERMAL_CONDUCTIVITY,
    SILICATE_SOLIDUS_BASE_K,
};
use crate::units::{HeatFlux, Length, Mass, Temperature};

pub fn crust_thermal_conductivity(kind: PlanetKind) -> f64 {
    match kind {
        PlanetKind::Telluric | PlanetKind::Chthonian => SILICATE_CRUST_THERMAL_CONDUCTIVITY,
        PlanetKind::CarbonPlanet => CARBON_CRUST_THERMAL_CONDUCTIVITY,
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet => {
            ICE_CRUST_THERMAL_CONDUCTIVITY
        }
        PlanetKind::GasGiant | PlanetKind::Exotic => SILICATE_CRUST_THERMAL_CONDUCTIVITY,
    }
}

pub fn mantle_solidus_temperature(kind: PlanetKind, mass: Mass) -> Temperature {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Temperature::new(0.0);
    }

    let base_solidus = match kind {
        PlanetKind::Telluric | PlanetKind::Chthonian => SILICATE_SOLIDUS_BASE_K,
        PlanetKind::CarbonPlanet => CARBON_SOLIDUS_BASE_K,
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet => ICE_SOLIDUS_BASE_K,
        PlanetKind::GasGiant | PlanetKind::Exotic => SILICATE_SOLIDUS_BASE_K,
    };

    let mass_ratio = m / EARTH_MASS;
    let scaling = (1.0 + 0.05 * (1.0 + mass_ratio).ln().max(0.0)).max(0.1);
    let t_solidus = base_solidus * scaling;

    if !t_solidus.is_finite() || t_solidus <= 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(t_solidus)
    }
}

pub fn lithosphere_thickness(
    base_temperature: Temperature,
    surface_temperature: Temperature,
    surface_heat_flux: HeatFlux,
    thermal_conductivity: f64,
) -> Length {
    let t_base = base_temperature.value();
    let t_surf = surface_temperature.value();
    let q = surface_heat_flux.value();
    let k = thermal_conductivity;

    if !t_base.is_finite()
        || !t_surf.is_finite()
        || !q.is_finite()
        || !k.is_finite()
        || k <= 0.0
        || t_base <= t_surf
    {
        return Length::new(0.0);
    }

    if q <= 0.0 {
        return Length::new(f64::INFINITY);
    }

    let delta_t = t_base - t_surf;
    let z_l = k * (delta_t / q);

    if !z_l.is_finite() || z_l <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(z_l)
    }
}

pub fn lithosphere_thickness_for_planet(
    kind: PlanetKind,
    mass: Mass,
    surface_temperature: Temperature,
    surface_heat_flux: HeatFlux,
) -> Length {
    let t_solidus = mantle_solidus_temperature(kind, mass);
    let k = crust_thermal_conductivity(kind);
    lithosphere_thickness(t_solidus, surface_temperature, surface_heat_flux, k)
}