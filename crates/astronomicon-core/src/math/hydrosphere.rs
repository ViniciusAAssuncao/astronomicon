use crate::chemistry::solvent::SolventProperties;
use crate::units::{Acceleration, Density, HeatFlux, Length, Mass, Pressure, Temperature};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HydrosphereStructure {
    pub total_volume_m3: f64,
    pub total_mass: Mass,
    pub ice_thickness: Length,
    pub liquid_depth: Length,
    pub is_subsurface_ocean: bool,
    pub is_completely_frozen: bool,
    pub is_completely_liquid: bool,
}

pub fn spherical_shell_volume(
    outer_radius: Length,
    shell_thickness: Length,
    coverage_fraction: f64,
) -> f64 {
    let r = outer_radius.value();
    let d = shell_thickness.value();
    let cov = coverage_fraction.clamp(0.0, 1.0);

    if r <= 0.0 || d <= 0.0 || cov <= 0.0 || !r.is_finite() || !d.is_finite() || !cov.is_finite() {
        return 0.0;
    }

    let r_in = (r - d).max(0.0);
    let full_volume = (4.0 / 3.0) * PI * (r.powi(3) - r_in.powi(3));
    cov * full_volume
}

pub fn hydrosphere_mass(
    planet_radius: Length,
    average_depth: Length,
    coverage_fraction: f64,
    liquid_density: Density,
    solute_mass_fraction: f64,
) -> Mass {
    let volume = spherical_shell_volume(planet_radius, average_depth, coverage_fraction);
    let rho = liquid_density.value();

    if volume <= 0.0 || rho <= 0.0 || !rho.is_finite() {
        return Mass::new(0.0);
    }

    let solvent_mass = volume * rho;
    let w = solute_mass_fraction.clamp(0.0, 0.999);
    let total_mass = solvent_mass / (1.0 - w);

    if !total_mass.is_finite() || total_mass < 0.0 {
        Mass::new(0.0)
    } else {
        Mass::new(total_mass)
    }
}

pub fn hydrostatic_pressure_at_depth(
    density: Density,
    gravity: Acceleration,
    depth: Length,
) -> Pressure {
    let rho = density.value();
    let g = gravity.value();
    let h = depth.value();

    if rho <= 0.0 || g <= 0.0 || h <= 0.0 || !rho.is_finite() || !g.is_finite() || !h.is_finite() {
        return Pressure::new(0.0);
    }

    Pressure::new(rho * g * h)
}

pub fn equilibrium_ice_thickness(
    surface_temperature: Temperature,
    freezing_point: Temperature,
    geothermal_heat_flux: HeatFlux,
    ice_thermal_conductivity: f64,
) -> Length {
    let t_surf = surface_temperature.value();
    let t_freeze = freezing_point.value();
    let q_geo = geothermal_heat_flux.value();
    let k = ice_thermal_conductivity;

    if t_surf >= t_freeze {
        return Length::new(0.0);
    }

    if q_geo <= 0.0 || !q_geo.is_finite() {
        return Length::new(f64::INFINITY);
    }

    if k <= 0.0 || !k.is_finite() {
        return Length::new(0.0);
    }

    let delta_t = t_freeze - t_surf;
    if !delta_t.is_finite() || delta_t <= 0.0 {
        return Length::new(0.0);
    }

    let h_ice = (k * delta_t) / q_geo;
    if !h_ice.is_finite() || h_ice < 0.0 {
        Length::new(0.0)
    } else {
        Length::new(h_ice)
    }
}

pub fn analyze_hydrosphere_structure(
    planet_radius: Length,
    average_depth: Length,
    coverage_fraction: f64,
    surface_temperature: Temperature,
    freezing_point: Temperature,
    geothermal_heat_flux: HeatFlux,
    properties: &SolventProperties,
    solute_mass_fraction: f64,
) -> HydrosphereStructure {
    let total_volume = spherical_shell_volume(planet_radius, average_depth, coverage_fraction);
    let total_mass = hydrosphere_mass(
        planet_radius,
        average_depth,
        coverage_fraction,
        properties.liquid_density,
        solute_mass_fraction,
    );

    let d = average_depth.value();
    let h_ice_eq = equilibrium_ice_thickness(
        surface_temperature,
        freezing_point,
        geothermal_heat_flux,
        properties.solid_thermal_conductivity,
    );

    let h_ice_val = h_ice_eq.value();

    if h_ice_val <= 0.0 {
        HydrosphereStructure {
            total_volume_m3: total_volume,
            total_mass,
            ice_thickness: Length::new(0.0),
            liquid_depth: average_depth,
            is_subsurface_ocean: false,
            is_completely_frozen: false,
            is_completely_liquid: true,
        }
    } else if h_ice_val >= d {
        HydrosphereStructure {
            total_volume_m3: total_volume,
            total_mass,
            ice_thickness: average_depth,
            liquid_depth: Length::new(0.0),
            is_subsurface_ocean: false,
            is_completely_frozen: true,
            is_completely_liquid: false,
        }
    } else {
        HydrosphereStructure {
            total_volume_m3: total_volume,
            total_mass,
            ice_thickness: Length::new(h_ice_val),
            liquid_depth: Length::new(d - h_ice_val),
            is_subsurface_ocean: true,
            is_completely_frozen: false,
            is_completely_liquid: false,
        }
    }
}
