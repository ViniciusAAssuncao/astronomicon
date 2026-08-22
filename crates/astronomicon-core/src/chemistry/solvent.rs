use crate::error::{DomainError, DomainResult};
use crate::units::{Pressure, Temperature};
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
        )),
        _ => None,
    }
}

pub fn mean_solvent_properties(composition: &[(String, f64)]) -> DomainResult<SolventProperties> {
    let total_percentage: f64 = composition.iter().map(|(_, p)| p).sum();

    if total_percentage <= 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: "total percentage must be positive".to_string(),
        });
    }

    let mut h_vap = 0.0;
    let mut h_fus = 0.0;
    let mut k_f = 0.0;
    let mut t_triple = 0.0;
    let mut p_triple = 0.0;
    let mut t_crit = 0.0;
    let mut p_crit = 0.0;
    let mut t_boil = 0.0;
    let mut t_melt = 0.0;

    for (formula, percentage) in composition {
        let props = solvent_properties_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: format!("unknown solvent formula '{}'", formula),
        })?;

        let fraction = percentage / total_percentage;
        h_vap += props.enthalpy_of_vaporization * fraction;
        h_fus += props.enthalpy_of_fusion * fraction;
        k_f += props.cryoscopic_constant * fraction;
        t_triple += props.triple_point_temperature.value() * fraction;
        p_triple += props.triple_point_pressure.value() * fraction;
        t_crit += props.critical_temperature.value() * fraction;
        p_crit += props.critical_pressure.value() * fraction;
        t_boil += props.normal_boiling_point.value() * fraction;
        t_melt += props.normal_melting_point.value() * fraction;
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
