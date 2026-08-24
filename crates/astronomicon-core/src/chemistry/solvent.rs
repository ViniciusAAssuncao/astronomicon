use crate::chemistry::composition_mean::validate_and_normalize_composition;
use crate::error::{DomainError, DomainResult};
use crate::units::{Density, Pressure, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SolventProperties {
    pub enthalpy_of_vaporization: f64,
    pub enthalpy_of_fusion: f64,
    pub cryoscopic_constant: f64,
    pub triple_point_temperature: Temperature,
    pub triple_point_pressure: Pressure,
    pub critical_temperature: Temperature,
    pub critical_pressure: Pressure,
    pub normal_boiling_point: Temperature,
    pub normal_melting_point: Temperature,
    pub liquid_density: Density,
    pub solid_density: Density,
    pub solid_thermal_conductivity: f64,
    pub liquid_specific_heat_capacity: f64,
    pub liquid_albedo: f64,
    pub solid_albedo: f64,
    pub liquid_refractive_index_real: f64,
    pub liquid_refractive_index_imag: f64,
    pub solid_refractive_index_real: f64,
    pub solid_refractive_index_imag: f64,
}

impl SolventProperties {
    pub fn new(
        enthalpy_of_vaporization: f64,
        enthalpy_of_fusion: f64,
        cryoscopic_constant: f64,
        triple_point_temperature: Temperature,
        triple_point_pressure: Pressure,
        critical_temperature: Temperature,
        critical_pressure: Pressure,
        normal_boiling_point: Temperature,
        normal_melting_point: Temperature,
        liquid_density: Density,
        solid_density: Density,
        solid_thermal_conductivity: f64,
        liquid_specific_heat_capacity: f64,
        liquid_albedo: f64,
        solid_albedo: f64,
        liquid_refractive_index_real: f64,
        liquid_refractive_index_imag: f64,
        solid_refractive_index_real: f64,
        solid_refractive_index_imag: f64,
    ) -> Self {
        Self {
            enthalpy_of_vaporization,
            enthalpy_of_fusion,
            cryoscopic_constant,
            triple_point_temperature,
            triple_point_pressure,
            critical_temperature,
            critical_pressure,
            normal_boiling_point,
            normal_melting_point,
            liquid_density,
            solid_density,
            solid_thermal_conductivity,
            liquid_specific_heat_capacity,
            liquid_albedo,
            solid_albedo,
            liquid_refractive_index_real,
            liquid_refractive_index_imag,
            solid_refractive_index_real,
            solid_refractive_index_imag,
        }
    }
}

pub fn solvent_properties_of(formula: &str) -> Option<SolventProperties> {
    match formula {
        "H2O" => Some(SolventProperties::new(
            40660.0,
            6010.0,
            1.853,
            Temperature::new(273.16),
            Pressure::new(611.657),
            Temperature::new(647.096),
            Pressure::new(22.064e6),
            Temperature::new(373.15),
            Temperature::new(273.15),
            Density::new(1000.0),
            Density::new(917.0),
            2.2,
            4184.0,
            0.06,
            0.65,
            1.333,
            1.0e-8,
            1.310,
            1.0e-8,
        )),
        "CH4" => Some(SolventProperties::new(
            8170.0,
            941.0,
            1.166,
            Temperature::new(90.69),
            Pressure::new(11696.0),
            Temperature::new(190.56),
            Pressure::new(4.5992e6),
            Temperature::new(111.66),
            Temperature::new(90.69),
            Density::new(422.8),
            Density::new(490.0),
            0.3,
            3400.0,
            0.10,
            0.50,
            1.280,
            1.0e-7,
            1.320,
            1.0e-7,
        )),
        "NH3" => Some(SolventProperties::new(
            23350.0,
            5660.0,
            0.97,
            Temperature::new(195.40),
            Pressure::new(6060.0),
            Temperature::new(405.40),
            Pressure::new(11.333e6),
            Temperature::new(239.82),
            Temperature::new(195.42),
            Density::new(681.9),
            Density::new(817.0),
            0.5,
            4700.0,
            0.08,
            0.65,
            1.330,
            1.0e-6,
            1.350,
            1.0e-6,
        )),
        "N2" => Some(SolventProperties::new(
            5560.0,
            720.0,
            1.99,
            Temperature::new(63.15),
            Pressure::new(12520.0),
            Temperature::new(126.21),
            Pressure::new(3.39e6),
            Temperature::new(77.36),
            Temperature::new(63.15),
            Density::new(808.0),
            Density::new(947.0),
            0.25,
            2040.0,
            0.10,
            0.70,
            1.200,
            1.0e-9,
            1.250,
            1.0e-9,
        )),
        "CO2" => Some(SolventProperties::new(
            15300.0,
            9020.0,
            3.70,
            Temperature::new(216.58),
            Pressure::new(518500.0),
            Temperature::new(304.13),
            Pressure::new(7.3773e6),
            Temperature::new(216.58),
            Temperature::new(216.58),
            Density::new(1101.0),
            Density::new(1562.0),
            0.6,
            2200.0,
            0.10,
            0.75,
            1.200,
            1.0e-8,
            1.410,
            1.0e-8,
        )),
        _ => None,
    }
}

pub fn mean_solvent_properties(composition: &[(String, f64)]) -> DomainResult<SolventProperties> {
    let fractions = validate_and_normalize_composition(
        composition,
        "composition",
        "total percentage must be positive",
    )?;

    let mut h_vap = 0.0;
    let mut h_fus = 0.0;
    let mut k_f = 0.0;
    let mut t_triple = 0.0;
    let mut p_triple = 0.0;
    let mut t_crit = 0.0;
    let mut p_crit = 0.0;
    let mut t_boil = 0.0;
    let mut t_melt = 0.0;
    let mut rho_liq = 0.0;
    let mut rho_sol = 0.0;
    let mut k_therm = 0.0;
    let mut cp_liq = 0.0;
    let mut alb_liq = 0.0;
    let mut alb_sol = 0.0;
    let mut n_liq_r = 0.0;
    let mut n_liq_i = 0.0;
    let mut n_sol_r = 0.0;
    let mut n_sol_i = 0.0;

    for (formula, fraction) in fractions {
        let props =
            solvent_properties_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
                field: "composition".to_string(),
                reason: format!("unknown solvent formula '{}'", formula),
            })?;

        h_vap += props.enthalpy_of_vaporization * fraction;
        h_fus += props.enthalpy_of_fusion * fraction;
        k_f += props.cryoscopic_constant * fraction;
        t_triple += props.triple_point_temperature.value() * fraction;
        p_triple += props.triple_point_pressure.value() * fraction;
        t_crit += props.critical_temperature.value() * fraction;
        p_crit += props.critical_pressure.value() * fraction;
        t_boil += props.normal_boiling_point.value() * fraction;
        t_melt += props.normal_melting_point.value() * fraction;
        rho_liq += props.liquid_density.value() * fraction;
        rho_sol += props.solid_density.value() * fraction;
        k_therm += props.solid_thermal_conductivity * fraction;
        cp_liq += props.liquid_specific_heat_capacity * fraction;
        alb_liq += props.liquid_albedo * fraction;
        alb_sol += props.solid_albedo * fraction;
        n_liq_r += props.liquid_refractive_index_real * fraction;
        n_liq_i += props.liquid_refractive_index_imag * fraction;
        n_sol_r += props.solid_refractive_index_real * fraction;
        n_sol_i += props.solid_refractive_index_imag * fraction;
    }

    Ok(SolventProperties::new(
        h_vap,
        h_fus,
        k_f,
        Temperature::new(t_triple),
        Pressure::new(p_triple),
        Temperature::new(t_crit),
        Pressure::new(p_crit),
        Temperature::new(t_boil),
        Temperature::new(t_melt),
        Density::new(rho_liq),
        Density::new(rho_sol),
        k_therm,
        cp_liq,
        alb_liq,
        alb_sol,
        n_liq_r,
        n_liq_i,
        n_sol_r,
        n_sol_i,
    ))
}

pub fn enthalpy_of_vaporization_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.enthalpy_of_vaporization)
}

pub fn enthalpy_of_fusion_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.enthalpy_of_fusion)
}

pub fn cryoscopic_constant_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.cryoscopic_constant)
}

pub fn triple_point_of(formula: &str) -> Option<(Temperature, Pressure)> {
    solvent_properties_of(formula).map(|p| (p.triple_point_temperature, p.triple_point_pressure))
}

pub fn critical_point_of(formula: &str) -> Option<(Temperature, Pressure)> {
    solvent_properties_of(formula).map(|p| (p.critical_temperature, p.critical_pressure))
}

pub fn liquid_density_of(formula: &str) -> Option<Density> {
    solvent_properties_of(formula).map(|p| p.liquid_density)
}

pub fn solid_density_of(formula: &str) -> Option<Density> {
    solvent_properties_of(formula).map(|p| p.solid_density)
}

pub fn solid_thermal_conductivity_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.solid_thermal_conductivity)
}

pub fn liquid_specific_heat_capacity_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.liquid_specific_heat_capacity)
}

pub fn liquid_albedo_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.liquid_albedo)
}

pub fn solid_albedo_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.solid_albedo)
}

pub fn liquid_refractive_index_real_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.liquid_refractive_index_real)
}

pub fn liquid_refractive_index_imag_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.liquid_refractive_index_imag)
}

pub fn solid_refractive_index_real_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.solid_refractive_index_real)
}

pub fn solid_refractive_index_imag_of(formula: &str) -> Option<f64> {
    solvent_properties_of(formula).map(|p| p.solid_refractive_index_imag)
}
