use crate::chemistry::solvent::SolventProperties;
use crate::math::aerosol::particle_terminal_velocity;
use crate::math::atmosphere::{ideal_gas_density, pressure_at_altitude};
use crate::math::climate::temperature_at_altitude;
use crate::math::clouds::relative_humidity_at_altitude;
use crate::math::thermodynamics::{
    dynamic_boiling_point, saturation_vapor_pressure, saturation_vapor_pressure_over_solid,
};
use crate::units::constants::{STANDARD_ATMOSPHERE_PRESSURE, UNIVERSAL_GAS_CONSTANT};
use crate::units::{
    Acceleration, Density, DynamicViscosity, Length, MolarMass, Pressure, Temperature,
    TemperatureGradient,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrecipitationPhase {
    Liquid,
    Solid,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfaceCondensationType {
    Dew,
    Frost,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SubcloudEvaporationResult {
    pub initial_radius: Length,
    pub final_radius: Length,
    pub mass_fraction_remaining: f64,
    pub is_virga: bool,
    pub evaporation_altitude: Option<Length>,
}

pub fn formation_precipitation_phase(
    cloud_temperature: Temperature,
    melting_point: Temperature,
) -> PrecipitationPhase {
    let t_c = cloud_temperature.value();
    let t_m = melting_point.value();

    if !t_c.is_finite() || !t_m.is_finite() {
        return PrecipitationPhase::Liquid;
    }

    if t_c > t_m {
        PrecipitationPhase::Liquid
    } else {
        PrecipitationPhase::Solid
    }
}

pub fn formation_precipitation_phase_at_altitude(
    surface_temperature: Temperature,
    cloud_altitude: Length,
    lapse_rate: TemperatureGradient,
    melting_point: Temperature,
) -> PrecipitationPhase {
    let t_cloud = temperature_at_altitude(surface_temperature, cloud_altitude, lapse_rate);
    formation_precipitation_phase(t_cloud, melting_point)
}

pub fn scan_precipitation_phase(
    surface_temperature: Temperature,
    cloud_base_altitude: Length,
    lapse_rate: TemperatureGradient,
    melting_point: Temperature,
) -> PrecipitationPhase {
    let t_melt = melting_point.value();
    let z_base = cloud_base_altitude.value();
    let t_surf = surface_temperature.value();

    if !t_melt.is_finite() || !z_base.is_finite() || z_base <= 0.0 || !t_surf.is_finite() {
        if t_surf <= t_melt {
            return PrecipitationPhase::Solid;
        } else {
            return PrecipitationPhase::Liquid;
        }
    }

    let t_cloud_base =
        temperature_at_altitude(surface_temperature, cloud_base_altitude, lapse_rate).value();

    let steps = 50;
    let dz = z_base / (steps as f64);

    let initial_solid = t_cloud_base <= t_melt;
    let mut has_melted = !initial_solid;
    let mut refrozen = false;

    for i in (0..=steps).rev() {
        let z = (i as f64) * dz;
        let t_z = temperature_at_altitude(surface_temperature, Length::new(z), lapse_rate).value();

        if initial_solid && !has_melted && t_z > t_melt {
            has_melted = true;
        } else if has_melted && t_z <= t_melt {
            refrozen = true;
        }
    }

    if refrozen {
        PrecipitationPhase::Mixed
    } else if has_melted || t_surf > t_melt {
        PrecipitationPhase::Liquid
    } else {
        PrecipitationPhase::Solid
    }
}

pub fn subcloud_evaporation_profile(
    initial_radius: Length,
    cloud_base_altitude: Length,
    surface_temperature: Temperature,
    surface_pressure: Pressure,
    surface_relative_humidity: f64,
    lapse_rate: TemperatureGradient,
    scale_height: Length,
    tropopause_altitude: Length,
    gravity: Acceleration,
    dynamic_viscosity: DynamicViscosity,
    solvent_properties: &SolventProperties,
    solvent_molar_mass: MolarMass,
    atmospheric_molar_mass: MolarMass,
    phase: PrecipitationPhase,
) -> SubcloudEvaporationResult {
    let r_init = initial_radius.value();
    let z_base = cloud_base_altitude.value();

    if !r_init.is_finite() || r_init <= 0.0 {
        return SubcloudEvaporationResult {
            initial_radius,
            final_radius: Length::new(0.0),
            mass_fraction_remaining: 0.0,
            is_virga: true,
            evaporation_altitude: Some(cloud_base_altitude),
        };
    }

    if !z_base.is_finite() || z_base <= 0.0 {
        return SubcloudEvaporationResult {
            initial_radius,
            final_radius: initial_radius,
            mass_fraction_remaining: 1.0,
            is_virga: false,
            evaporation_altitude: None,
        };
    }

    let rho_p = match phase {
        PrecipitationPhase::Solid => solvent_properties.solid_density,
        _ => solvent_properties.liquid_density,
    };
    let rho_p_val = if rho_p.value() > 0.0 && rho_p.value().is_finite() {
        rho_p.value()
    } else {
        1000.0
    };

    let mut r = r_init;
    let mut m = (4.0 / 3.0) * PI * rho_p_val * r.powi(3);
    let m_initial = m;

    let steps = ((z_base / 20.0).ceil() as usize).clamp(20, 500);
    let dz = z_base / (steps as f64);

    for i in 0..steps {
        let z = z_base - ((i as f64) + 0.5) * dz;
        let alt = Length::new(z);

        let t_z = temperature_at_altitude(surface_temperature, alt, lapse_rate);
        let p_z = pressure_at_altitude(surface_pressure, alt, scale_height);

        let t_val = t_z.value();
        let p_val = p_z.value();

        if t_val <= 0.0 || p_val <= 0.0 || !t_val.is_finite() || !p_val.is_finite() {
            continue;
        }

        let t_boil = dynamic_boiling_point(p_z, solvent_properties);
        if t_val >= t_boil.value() {
            return SubcloudEvaporationResult {
                initial_radius,
                final_radius: Length::new(0.0),
                mass_fraction_remaining: 0.0,
                is_virga: true,
                evaporation_altitude: Some(alt),
            };
        }

        let rh = relative_humidity_at_altitude(surface_relative_humidity, alt, tropopause_altitude);
        let p_sat = match phase {
            PrecipitationPhase::Solid => {
                saturation_vapor_pressure_over_solid(t_z, solvent_properties)
            }
            _ => saturation_vapor_pressure(t_z, solvent_properties),
        };

        let delta_pv = (p_sat.value() * (1.0 - rh)).max(0.0);
        if delta_pv <= 0.0 {
            continue;
        }

        let rho_fluid = ideal_gas_density(p_z, atmospheric_molar_mass, t_z);
        let v_term = particle_terminal_velocity(
            gravity,
            Density::new(rho_p_val),
            rho_fluid,
            Length::new(r),
            dynamic_viscosity,
        )
        .value();

        let v_fall = v_term.max(0.01);
        let dt = dz / v_fall;

        let d_v = 2.2e-5
            * (t_val / 273.15).powf(1.75)
            * (STANDARD_ATMOSPHERE_PRESSURE / p_val.max(1.0));

        let eta = dynamic_viscosity.value().max(1e-7);
        let rho_f_val = rho_fluid.value().max(1e-6);
        let re = (2.0 * rho_f_val * v_fall * r) / eta;
        let sc = eta / (rho_f_val * d_v).max(1e-12);
        let f_v = (1.0 + 0.3 * re.max(0.0).sqrt() * sc.max(0.0).cbrt()).clamp(1.0, 50.0);

        let dm_dt = (4.0 * PI * r * d_v * solvent_molar_mass.value() * delta_pv * f_v)
            / (UNIVERSAL_GAS_CONSTANT * t_val);

        let delta_m = dm_dt * dt;
        if delta_m >= m {
            return SubcloudEvaporationResult {
                initial_radius,
                final_radius: Length::new(0.0),
                mass_fraction_remaining: 0.0,
                is_virga: true,
                evaporation_altitude: Some(alt),
            };
        }

        m -= delta_m;
        r = ((3.0 * m) / (4.0 * PI * rho_p_val)).cbrt();
    }

    let mass_frac = (m / m_initial).clamp(0.0, 1.0);
    let is_virga = mass_frac <= 1e-4 || r <= 1e-7;

    SubcloudEvaporationResult {
        initial_radius,
        final_radius: if is_virga {
            Length::new(0.0)
        } else {
            Length::new(r)
        },
        mass_fraction_remaining: if is_virga { 0.0 } else { mass_frac },
        is_virga,
        evaporation_altitude: if is_virga {
            Some(Length::new(0.0))
        } else {
            None
        },
    }
}

pub fn classify_surface_condensation(
    surface_dew_point: Temperature,
    melting_point: Temperature,
) -> SurfaceCondensationType {
    if surface_dew_point.value() < melting_point.value() {
        SurfaceCondensationType::Frost
    } else {
        SurfaceCondensationType::Dew
    }
}
