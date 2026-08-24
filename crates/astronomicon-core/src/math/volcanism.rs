use crate::domain::{PlanetKind, TectonicRegime};
use crate::units::constants::{
    CO2_HENRY_SOLUBILITY_COEFFICIENT, H2O_HENRY_SOLUBILITY_COEFFICIENT,
    SILICATE_LATENT_HEAT_OF_FUSION, SILICATE_MELT_SPECIFIC_HEAT, SILICATE_MELT_THERMAL_EXPANSION,
    SO2_HENRY_SOLUBILITY_COEFFICIENT,
};
use crate::units::{
    Acceleration, Density, HeatFlux, Length, MassRate, Pressure, Speed, Temperature,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolcanicEruptionStyle {
    Effusive,
    Explosive,
    SubaqueousEffusive,
    Cryovolcanic,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolcanicGasOutgassingRates {
    pub h2o: MassRate,
    pub co2: MassRate,
    pub so2: MassRate,
    pub h2s: MassRate,
    pub total: MassRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MagmaProperties {
    pub temperature: Temperature,
    pub density: Density,
    pub dynamic_viscosity_pa_s: f64,
    pub melt_fraction: f64,
    pub dissolved_volatile_fraction: f64,
}

pub fn lithospheric_base_pressure(
    surface_gravity: Acceleration,
    mantle_density: Density,
    lithosphere_thickness: Length,
) -> Pressure {
    let g = surface_gravity.value();
    let rho = mantle_density.value();
    let z = lithosphere_thickness.value();

    if g <= 0.0 || rho <= 0.0 || z <= 0.0 || !g.is_finite() || !rho.is_finite() || !z.is_finite() {
        return Pressure::new(0.0);
    }

    Pressure::new(rho * g * z)
}

pub fn depressed_solidus_temperature(
    dry_solidus: Temperature,
    mantle_hydration_fraction: f64,
) -> Temperature {
    let t_dry = dry_solidus.value();
    if !t_dry.is_finite() || t_dry <= 0.0 {
        return Temperature::new(0.0);
    }

    let h = mantle_hydration_fraction.clamp(0.0, 1.0);
    if h <= 0.0 {
        return dry_solidus;
    }

    let delta_t_max = 450.0;
    let delta_t = delta_t_max * (1.0 - (-25.0 * h).exp());
    let t_wet = (t_dry - delta_t).max(273.15);

    Temperature::new(t_wet)
}

pub fn mantle_potential_temperature(
    solidus_temperature: Temperature,
    convective_heat_flux: HeatFlux,
) -> Temperature {
    let t_sol = solidus_temperature.value();
    let q = convective_heat_flux.value();

    if !t_sol.is_finite() || t_sol <= 0.0 {
        return Temperature::new(0.0);
    }

    if !q.is_finite() || q <= 0.0 {
        return solidus_temperature;
    }

    let q_ref = 0.04;
    let factor = (q / q_ref).sqrt().clamp(0.0, 4.0);
    let delta_t = 250.0 * factor;

    Temperature::new(t_sol + delta_t)
}

pub fn decompression_melting_temperature(
    base_temperature: Temperature,
    surface_gravity: Acceleration,
    lithosphere_thickness: Length,
    specific_heat_capacity: f64,
    thermal_expansion: f64,
) -> Temperature {
    let t_base = base_temperature.value();
    let g = surface_gravity.value();
    let z_l = lithosphere_thickness.value();
    let cp = specific_heat_capacity;
    let alpha = thermal_expansion;

    if t_base <= 0.0
        || g <= 0.0
        || z_l <= 0.0
        || cp <= 0.0
        || alpha <= 0.0
        || !t_base.is_finite()
        || !g.is_finite()
        || !z_l.is_finite()
        || !cp.is_finite()
        || !alpha.is_finite()
    {
        return base_temperature;
    }

    let adiabatic_gradient = (alpha * g * t_base) / cp;
    let z_ascent = z_l * 0.9;
    let t_ascent = t_base - adiabatic_gradient * z_ascent;

    Temperature::new(t_ascent.max(0.0))
}

pub fn partial_melt_fraction(
    local_temperature: Temperature,
    solidus: Temperature,
    liquidus: Temperature,
) -> f64 {
    let t = local_temperature.value();
    let t_sol = solidus.value();
    let t_liq = liquidus.value();

    if !t.is_finite() || !t_sol.is_finite() || !t_liq.is_finite() || t_liq <= t_sol || t <= t_sol {
        return 0.0;
    }

    if t >= t_liq {
        return 1.0;
    }

    ((t - t_sol) / (t_liq - t_sol)).clamp(0.0, 1.0)
}

pub fn cryovolcanic_melt_fraction(
    surface_temperature: Temperature,
    solvent_melting_point: Temperature,
    geothermal_heat_flux: HeatFlux,
    solute_fraction: f64,
    ice_thickness: Length,
    ice_thermal_conductivity: f64,
) -> (f64, Temperature) {
    let t_surf = surface_temperature.value();
    let t_melt_base = solvent_melting_point.value();
    let q_geo = geothermal_heat_flux.value();
    let w = solute_fraction.clamp(0.0, 0.999);
    let z_ice = ice_thickness.value();
    let k = ice_thermal_conductivity;

    if t_melt_base <= 0.0 || !t_surf.is_finite() || !t_melt_base.is_finite() {
        return (0.0, Temperature::new(0.0));
    }

    let depression = 60.0 * (1.0 - (-10.0 * w).exp());
    let t_freeze = (t_melt_base - depression).max(50.0);

    let basal_temp = if q_geo > 0.0
        && k > 0.0
        && z_ice > 0.0
        && q_geo.is_finite()
        && k.is_finite()
        && z_ice.is_finite()
    {
        t_surf + (q_geo * z_ice) / k
    } else {
        t_surf
    };

    if basal_temp <= t_freeze {
        (0.0, Temperature::new(basal_temp))
    } else {
        let delta_t_range = 30.0;
        let frac = ((basal_temp - t_freeze) / delta_t_range).clamp(0.0, 1.0);
        (frac, Temperature::new(basal_temp))
    }
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

pub fn heat_pipe_extrusion_rate(
    total_surface_heat_flux: HeatFlux,
    planet_radius: Length,
    latent_heat_of_fusion: f64,
    specific_heat_capacity: f64,
    magma_temperature: Temperature,
    surface_temperature: Temperature,
) -> MassRate {
    let q = total_surface_heat_flux.value();
    let r = planet_radius.value();
    let l_f = if latent_heat_of_fusion > 0.0 && latent_heat_of_fusion.is_finite() {
        latent_heat_of_fusion
    } else {
        SILICATE_LATENT_HEAT_OF_FUSION
    };
    let cp = if specific_heat_capacity > 0.0 && specific_heat_capacity.is_finite() {
        specific_heat_capacity
    } else {
        SILICATE_MELT_SPECIFIC_HEAT
    };
    let t_m = magma_temperature.value();
    let t_s = surface_temperature.value();

    if q <= 0.0
        || r <= 0.0
        || l_f <= 0.0
        || cp <= 0.0
        || !q.is_finite()
        || !r.is_finite()
        || !l_f.is_finite()
        || !cp.is_finite()
    {
        return MassRate::new(0.0);
    }

    let area = 4.0 * PI * r * r;
    let total_heat_loss = q * area;
    let delta_t = (t_m - t_s).max(0.0);
    let enthalpy_per_kg = l_f + cp * delta_t;

    if enthalpy_per_kg <= 0.0 {
        return MassRate::new(0.0);
    }

    MassRate::new(total_heat_loss / enthalpy_per_kg)
}

pub fn plate_tectonics_extrusion_rate(
    plate_velocity: Speed,
    plate_count: u32,
    planet_radius: Length,
    lithosphere_thickness: Length,
    mantle_density: Density,
    melt_fraction: f64,
    mantle_hydration: f64,
) -> MassRate {
    let v = plate_velocity.value();
    let r = planet_radius.value();
    let z_l = lithosphere_thickness.value();
    let rho = mantle_density.value();
    let phi = melt_fraction.clamp(0.0, 1.0);
    let h_frac = mantle_hydration.clamp(0.0, 1.0);

    if v <= 0.0
        || r <= 0.0
        || z_l <= 0.0
        || rho <= 0.0
        || phi <= 0.0
        || !v.is_finite()
        || !r.is_finite()
        || !z_l.is_finite()
        || !rho.is_finite()
    {
        return MassRate::new(0.0);
    }

    let n_plates = plate_count.max(2) as f64;
    let ridge_length = n_plates.sqrt() * PI * r * 0.5;
    let spreading_area_rate = ridge_length * v;
    let melt_column_thickness = (z_l * phi).min(r * 0.1);
    let ridge_mass_rate = rho * spreading_area_rate * melt_column_thickness * phi;

    let subduction_length = ridge_length;
    let convergence_area_rate = subduction_length * v;
    let subduction_factor = (h_frac / 0.02).clamp(0.0, 3.0);
    let subduction_mass_rate =
        rho * convergence_area_rate * melt_column_thickness * 0.25 * subduction_factor;

    MassRate::new(ridge_mass_rate + subduction_mass_rate)
}

pub fn stagnant_lid_extrusion_rate(
    cmb_heat_flux: HeatFlux,
    core_radius: Length,
    planet_radius: Length,
    lithosphere_thickness: Length,
    mantle_density: Density,
    lithosphere_yield_strength: Pressure,
    buoyancy_overpressure: Pressure,
    latent_heat_of_fusion: f64,
    specific_heat_capacity: f64,
    delta_temperature: Temperature,
) -> MassRate {
    let q_cmb = cmb_heat_flux.value();
    let r_c = core_radius.value();
    let r_p = planet_radius.value();
    let z_l = lithosphere_thickness.value();
    let _rho = mantle_density.value();
    let sigma_y = lithosphere_yield_strength.value();
    let delta_p = buoyancy_overpressure.value();
    let l_f = if latent_heat_of_fusion > 0.0 && latent_heat_of_fusion.is_finite() {
        latent_heat_of_fusion
    } else {
        SILICATE_LATENT_HEAT_OF_FUSION
    };
    let cp = if specific_heat_capacity > 0.0 && specific_heat_capacity.is_finite() {
        specific_heat_capacity
    } else {
        SILICATE_MELT_SPECIFIC_HEAT
    };
    let dt = delta_temperature.value().max(10.0);

    if q_cmb <= 0.0
        || r_c <= 0.0
        || r_p <= 0.0
        || z_l <= 0.0
        || l_f <= 0.0
        || cp <= 0.0
        || !q_cmb.is_finite()
        || !r_c.is_finite()
        || !r_p.is_finite()
        || !z_l.is_finite()
        || !l_f.is_finite()
        || !cp.is_finite()
    {
        return MassRate::new(0.0);
    }

    let area_cmb = 4.0 * PI * r_c * r_c;
    let plume_power = q_cmb * area_cmb * 0.2;
    let enthalpy = l_f + cp * dt;
    let potential_melt_rate = plume_power / enthalpy;

    let stress_ratio = if sigma_y > 0.0 {
        (delta_p / sigma_y).max(0.0)
    } else {
        1.0
    };
    let penetration_factor = stress_ratio.powf(1.5).clamp(0.01, 1.0);
    let thickness_attenuation = r_p / (r_p + 8.0 * z_l);

    MassRate::new(potential_melt_rate * penetration_factor * thickness_attenuation)
}

pub fn global_magma_extrusion_rate(
    regime: TectonicRegime,
    kind: PlanetKind,
    total_surface_heat_flux: HeatFlux,
    cmb_heat_flux: HeatFlux,
    planet_radius: Length,
    core_radius: Length,
    lithosphere_thickness: Length,
    plate_velocity: Speed,
    plate_count: u32,
    mantle_density: Density,
    crust_density: Density,
    magma_density: Density,
    surface_gravity: Acceleration,
    lithosphere_yield_strength: Pressure,
    melt_fraction: f64,
    mantle_hydration: f64,
    latent_heat_of_fusion: f64,
    specific_heat_capacity: f64,
    magma_temperature: Temperature,
    surface_temperature: Temperature,
) -> MassRate {
    match regime {
        TectonicRegime::HeatPipe => heat_pipe_extrusion_rate(
            total_surface_heat_flux,
            planet_radius,
            latent_heat_of_fusion,
            specific_heat_capacity,
            magma_temperature,
            surface_temperature,
        ),
        TectonicRegime::PlateTectonics => plate_tectonics_extrusion_rate(
            plate_velocity,
            plate_count,
            planet_radius,
            lithosphere_thickness,
            mantle_density,
            melt_fraction,
            mantle_hydration,
        ),
        TectonicRegime::StagnantLid => {
            let overpressure = buoyancy_overpressure(
                crust_density,
                magma_density,
                surface_gravity,
                lithosphere_thickness,
            );
            let dt = Temperature::new(
                (magma_temperature.value() - surface_temperature.value()).max(0.0),
            );
            stagnant_lid_extrusion_rate(
                cmb_heat_flux,
                core_radius,
                planet_radius,
                lithosphere_thickness,
                mantle_density,
                lithosphere_yield_strength,
                overpressure,
                latent_heat_of_fusion,
                specific_heat_capacity,
                dt,
            )
        }
        TectonicRegime::IceTectonics => {
            if matches!(
                kind,
                PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet
            ) {
                plate_tectonics_extrusion_rate(
                    plate_velocity,
                    plate_count,
                    planet_radius,
                    lithosphere_thickness,
                    mantle_density,
                    melt_fraction,
                    mantle_hydration,
                )
            } else {
                MassRate::new(0.0)
            }
        }
        TectonicRegime::Inactive => MassRate::new(0.0),
    }
}

pub fn henry_solubility_h2o(surface_pressure: Pressure) -> f64 {
    let p = surface_pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return 0.0;
    }
    (H2O_HENRY_SOLUBILITY_COEFFICIENT * p.sqrt()).min(0.2)
}

pub fn henry_solubility_co2(surface_pressure: Pressure) -> f64 {
    let p = surface_pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return 0.0;
    }
    (CO2_HENRY_SOLUBILITY_COEFFICIENT * p).min(0.05)
}

pub fn henry_solubility_so2(surface_pressure: Pressure) -> f64 {
    let p = surface_pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return 0.0;
    }
    (SO2_HENRY_SOLUBILITY_COEFFICIENT * p.sqrt()).min(0.05)
}

pub fn exsolved_volatile_fraction(total_volatile_fraction: f64, solubility_limit: f64) -> f64 {
    if !total_volatile_fraction.is_finite() || total_volatile_fraction <= 0.0 {
        return 0.0;
    }
    (total_volatile_fraction - solubility_limit.max(0.0)).max(0.0)
}

pub fn classify_eruption_style(
    magma_viscosity_pa_s: f64,
    surface_pressure: Pressure,
    surface_gravity: Acceleration,
    exsolved_gas_mass_fraction: f64,
    is_subaqueous: bool,
    kind: PlanetKind,
    extrusion_rate: MassRate,
) -> VolcanicEruptionStyle {
    if extrusion_rate.value() <= 0.0 || !extrusion_rate.value().is_finite() {
        return VolcanicEruptionStyle::Inactive;
    }

    if matches!(
        kind,
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet
    ) {
        return VolcanicEruptionStyle::Cryovolcanic;
    }

    if is_subaqueous && surface_pressure.value() > 3.0e6 {
        return VolcanicEruptionStyle::SubaqueousEffusive;
    }

    let mu = magma_viscosity_pa_s;
    let p = surface_pressure.value();
    let g = surface_gravity.value();
    let x_g = exsolved_gas_mass_fraction.clamp(0.0, 1.0);

    if !mu.is_finite() || !p.is_finite() || !g.is_finite() {
        return VolcanicEruptionStyle::Effusive;
    }

    let fragmentation_index = (mu * x_g * g) / (p + 1.0e4);
    if fragmentation_index > 0.5 && x_g > 0.005 && mu > 1.0e3 {
        VolcanicEruptionStyle::Explosive
    } else {
        VolcanicEruptionStyle::Effusive
    }
}

pub fn volcanic_outgassing_fluxes(
    magma_extrusion_rate: MassRate,
    mantle_hydration: f64,
    c_o_ratio: f64,
    sulfur_mass_fraction: f64,
    surface_pressure: Pressure,
) -> VolcanicGasOutgassingRates {
    let m_dot = magma_extrusion_rate.value();
    if m_dot <= 0.0 || !m_dot.is_finite() {
        return VolcanicGasOutgassingRates {
            h2o: MassRate::new(0.0),
            co2: MassRate::new(0.0),
            so2: MassRate::new(0.0),
            h2s: MassRate::new(0.0),
            total: MassRate::new(0.0),
        };
    }

    let sol_h2o = henry_solubility_h2o(surface_pressure);
    let sol_co2 = henry_solubility_co2(surface_pressure);
    let sol_so2 = henry_solubility_so2(surface_pressure);

    let total_h2o = mantle_hydration.clamp(0.0, 0.1);
    let ex_h2o = exsolved_volatile_fraction(total_h2o, sol_h2o);

    let base_carbon_fraction = 0.002 * (c_o_ratio / 0.5).clamp(0.1, 5.0);
    let ex_c = exsolved_volatile_fraction(base_carbon_fraction, sol_co2);

    let total_s = sulfur_mass_fraction.clamp(0.0, 0.05);
    let ex_s = exsolved_volatile_fraction(total_s, sol_so2);

    let co2_fraction = if c_o_ratio > 0.8 {
        ex_c * 0.3
    } else {
        ex_c * 0.95
    };
    let (so2_fraction, h2s_fraction) = if c_o_ratio > 0.8 {
        (ex_s * 0.1, ex_s * 0.9)
    } else {
        (ex_s * 0.85, ex_s * 0.15)
    };

    let rate_h2o = m_dot * ex_h2o;
    let rate_co2 = m_dot * co2_fraction;
    let rate_so2 = m_dot * so2_fraction;
    let rate_h2s = m_dot * h2s_fraction;
    let total = rate_h2o + rate_co2 + rate_so2 + rate_h2s;

    VolcanicGasOutgassingRates {
        h2o: MassRate::new(rate_h2o),
        co2: MassRate::new(rate_co2),
        so2: MassRate::new(rate_so2),
        h2s: MassRate::new(rate_h2s),
        total: MassRate::new(total),
    }
}
