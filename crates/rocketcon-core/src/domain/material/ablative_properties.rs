use crate::error::RocketDomainResult;
use astronomicon_core::domain::validation::{
    validate_non_negative_finite, validate_positive_finite, validate_unit_interval,
};
use astronomicon_core::units::{SpecificEnergy, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AblativeMaterialProperties {
    heat_of_ablation: SpecificEnergy,
    char_yield_fraction: f64,
    recession_onset_temperature: Temperature,
    pyrolysis_gas_blowing_coefficient: f64,
    thermal_softening_exponent: Option<f64>,
}

impl AblativeMaterialProperties {
    pub fn new(
        heat_of_ablation: SpecificEnergy,
        char_yield_fraction: f64,
        recession_onset_temperature: Temperature,
        pyrolysis_gas_blowing_coefficient: f64,
        thermal_softening_exponent: Option<f64>,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(heat_of_ablation.value(), "heat_of_ablation")?;
        validate_unit_interval(char_yield_fraction, "char_yield_fraction")?;
        validate_positive_finite(
            recession_onset_temperature.value(),
            "recession_onset_temperature",
        )?;
        validate_non_negative_finite(
            pyrolysis_gas_blowing_coefficient,
            "pyrolysis_gas_blowing_coefficient",
        )?;

        if let Some(exp) = thermal_softening_exponent {
            validate_non_negative_finite(exp, "thermal_softening_exponent")?;
        }

        Ok(Self {
            heat_of_ablation,
            char_yield_fraction,
            recession_onset_temperature,
            pyrolysis_gas_blowing_coefficient,
            thermal_softening_exponent,
        })
    }

    pub fn new_simple(
        heat_of_ablation: SpecificEnergy,
        char_yield_fraction: f64,
        recession_onset_temperature: Temperature,
        pyrolysis_gas_blowing_coefficient: f64,
    ) -> RocketDomainResult<Self> {
        Self::new(
            heat_of_ablation,
            char_yield_fraction,
            recession_onset_temperature,
            pyrolysis_gas_blowing_coefficient,
            None,
        )
    }

    pub fn heat_of_ablation(&self) -> SpecificEnergy {
        self.heat_of_ablation
    }

    pub fn char_yield_fraction(&self) -> f64 {
        self.char_yield_fraction
    }

    pub fn recession_onset_temperature(&self) -> Temperature {
        self.recession_onset_temperature
    }

    pub fn recession_temperature_onset(&self) -> Temperature {
        self.recession_onset_temperature
    }

    pub fn pyrolysis_gas_blowing_coefficient(&self) -> f64 {
        self.pyrolysis_gas_blowing_coefficient
    }

    pub fn pyrolysis_blowing_coefficient(&self) -> f64 {
        self.pyrolysis_gas_blowing_coefficient
    }

    pub fn thermal_softening_exponent(&self) -> Option<f64> {
        self.thermal_softening_exponent
    }
}
