use crate::chemistry::solvent::{solvent_properties_of, SolventProperties};
use crate::domain::Hydrosphere;
use crate::error::{DomainError, DomainResult};
use crate::units::constants::UNIVERSAL_GAS_CONSTANT;
use crate::units::{Pressure, Temperature};
use serde::{Deserialize, Serialize};

pub const STANDARD_ATMOSPHERE_PRESSURE: f64 = 101_325.0;
pub const DEFAULT_SOLUTE_MOLAR_MASS_KG: f64 = 0.05844;
pub const DEFAULT_VAN_T_HOFF_FACTOR: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatterState {
    Solid,
    Liquid,
    Vapor,
    Supercritical,
}

pub fn boiling_point_clausius_clapeyron(
    pressure: Pressure,
    reference_boiling_point: Temperature,
    reference_pressure: Pressure,
    enthalpy_of_vaporization: f64,
) -> Temperature {
    let p = pressure.value();
    let p0 = reference_pressure.value();
    let t0 = reference_boiling_point.value();
    let delta_h = enthalpy_of_vaporization;

    if p <= 0.0
        || p0 <= 0.0
        || t0 <= 0.0
        || delta_h <= 0.0
        || !p.is_finite()
        || !p0.is_finite()
        || !t0.is_finite()
        || !delta_h.is_finite()
    {
        return Temperature::new(0.0);
    }

    let inv_t = (1.0 / t0) - (UNIVERSAL_GAS_CONSTANT / delta_h) * (p / p0).ln();

    if !inv_t.is_finite() || inv_t <= 0.0 {
        return Temperature::new(0.0);
    }

    Temperature::new(1.0 / inv_t)
}

pub fn dynamic_boiling_point(pressure: Pressure, properties: &SolventProperties) -> Temperature {
    let p = pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return Temperature::new(0.0);
    }

    let t_boil = boiling_point_clausius_clapeyron(
        pressure,
        properties.normal_boiling_point,
        Pressure::new(STANDARD_ATMOSPHERE_PRESSURE),
        properties.enthalpy_of_vaporization,
    );

    if t_boil.value() <= 0.0 || t_boil.value() > properties.critical_temperature.value() {
        properties.critical_temperature
    } else {
        t_boil
    }
}

pub fn dynamic_boiling_point_of(pressure: Pressure, formula: &str) -> DomainResult<Temperature> {
    let props = solvent_properties_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
        field: "formula".to_string(),
        reason: format!("unknown solvent formula '{}'", formula),
    })?;

    Ok(dynamic_boiling_point(pressure, &props))
}

pub fn freezing_point_depression(
    solute_mass_fraction: f64,
    cryoscopic_constant: f64,
    solute_molar_mass_kg_per_mol: f64,
    van_t_hoff_factor: f64,
) -> Temperature {
    if !solute_mass_fraction.is_finite()
        || solute_mass_fraction <= 0.0
        || !cryoscopic_constant.is_finite()
        || cryoscopic_constant <= 0.0
        || !solute_molar_mass_kg_per_mol.is_finite()
        || solute_molar_mass_kg_per_mol <= 0.0
        || !van_t_hoff_factor.is_finite()
        || van_t_hoff_factor <= 0.0
    {
        return Temperature::new(0.0);
    }

    let w = solute_mass_fraction.clamp(0.0, 0.999);
    let molality = w / ((1.0 - w) * solute_molar_mass_kg_per_mol);
    let delta_t = cryoscopic_constant * molality * van_t_hoff_factor;

    if !delta_t.is_finite() || delta_t < 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(delta_t)
    }
}

pub fn depressed_freezing_point(
    normal_melting_point: Temperature,
    solute_mass_fraction: f64,
    cryoscopic_constant: f64,
    solute_molar_mass_kg_per_mol: f64,
    van_t_hoff_factor: f64,
) -> Temperature {
    let delta_t = freezing_point_depression(
        solute_mass_fraction,
        cryoscopic_constant,
        solute_molar_mass_kg_per_mol,
        van_t_hoff_factor,
    );

    let t_freeze = normal_melting_point.value() - delta_t.value();
    Temperature::new(t_freeze.max(0.0))
}

pub fn determine_matter_state(
    temperature: Temperature,
    pressure: Pressure,
    properties: &SolventProperties,
    solute_mass_fraction: f64,
) -> MatterState {
    let t = temperature.value();
    let p = pressure.value();

    if !t.is_finite() || t <= 0.0 {
        return MatterState::Solid;
    }

    if !p.is_finite() || p <= 0.0 {
        return MatterState::Vapor;
    }

    if t >= properties.critical_temperature.value() && p >= properties.critical_pressure.value() {
        return MatterState::Supercritical;
    }

    if p < properties.triple_point_pressure.value() {
        let delta_h_subl = properties.enthalpy_of_vaporization + properties.enthalpy_of_fusion;
        let t_subl = boiling_point_clausius_clapeyron(
            pressure,
            properties.triple_point_temperature,
            properties.triple_point_pressure,
            delta_h_subl,
        );

        let t_subl_val = if t_subl.value() > 0.0 {
            t_subl.value()
        } else {
            properties.triple_point_temperature.value()
        };

        if t < t_subl_val {
            MatterState::Solid
        } else {
            MatterState::Vapor
        }
    } else {
        let t_freeze = depressed_freezing_point(
            properties.normal_melting_point,
            solute_mass_fraction,
            properties.cryoscopic_constant,
            DEFAULT_SOLUTE_MOLAR_MASS_KG,
            DEFAULT_VAN_T_HOFF_FACTOR,
        );

        let t_boil = dynamic_boiling_point(pressure, properties);

        if t < t_freeze.value() {
            MatterState::Solid
        } else if t > t_boil.value() {
            MatterState::Vapor
        } else {
            MatterState::Liquid
        }
    }
}

pub fn determine_matter_state_for_formula(
    temperature: Temperature,
    pressure: Pressure,
    formula: &str,
    solute_mass_fraction: f64,
) -> DomainResult<MatterState> {
    let props = solvent_properties_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
        field: "formula".to_string(),
        reason: format!("unknown solvent formula '{}'", formula),
    })?;

    Ok(determine_matter_state(
        temperature,
        pressure,
        &props,
        solute_mass_fraction,
    ))
}

pub fn determine_hydrosphere_state(
    temperature: Temperature,
    pressure: Pressure,
    hydrosphere: &Hydrosphere,
) -> DomainResult<MatterState> {
    let props = hydrosphere.mean_solvent_properties()?;
    Ok(determine_matter_state(
        temperature,
        pressure,
        &props,
        hydrosphere.salinity_or_solute_mass_fraction(),
    ))
}
