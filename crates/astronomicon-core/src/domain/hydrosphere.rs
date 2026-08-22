use crate::chemistry::molecular_formula::parse;
use crate::chemistry::solvent::{mean_solvent_properties, SolventProperties};
use crate::error::{DomainError, DomainResult};
use crate::math::thermodynamics::{
    depressed_freezing_point, determine_hydrosphere_state, dynamic_boiling_point, MatterState,
    DEFAULT_SOLUTE_MOLAR_MASS_KG, DEFAULT_VAN_T_HOFF_FACTOR,
};
use crate::units::constants::ATMOSPHERE_COMPOSITION_MAX_PERCENT_OVERAGE;
use crate::units::{Length, Pressure, Temperature};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HydrosphereComponent {
    formula: String,
    percentage: f64,
}

impl HydrosphereComponent {
    pub fn new(formula: String, percentage: f64) -> DomainResult<Self> {
        if !percentage.is_finite() || !(0.0..=100.0).contains(&percentage) {
            return Err(DomainError::InvalidInvariant {
                field: "percentage".to_string(),
                reason: "must be between 0.0 and 100.0".to_string(),
            });
        }

        parse(&formula)?;

        Ok(Self {
            formula,
            percentage,
        })
    }

    pub fn formula(&self) -> &str {
        &self.formula
    }

    pub fn percentage(&self) -> f64 {
        self.percentage
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hydrosphere {
    id: Uuid,
    planet_id: Uuid,
    average_depth: Length,
    surface_coverage_fraction: f64,
    salinity_or_solute_mass_fraction: f64,
    composition: Vec<HydrosphereComponent>,
}

impl Hydrosphere {
    pub fn new(
        id: Uuid,
        planet_id: Uuid,
        average_depth: Length,
        surface_coverage_fraction: f64,
        salinity_or_solute_mass_fraction: f64,
        composition: Vec<HydrosphereComponent>,
    ) -> DomainResult<Self> {
        if !average_depth.value().is_finite() || average_depth.value() < 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "average_depth".to_string(),
                reason: "must be finite and non-negative".to_string(),
            });
        }

        if !surface_coverage_fraction.is_finite()
            || !(0.0..=1.0).contains(&surface_coverage_fraction)
        {
            return Err(DomainError::InvalidInvariant {
                field: "surface_coverage_fraction".to_string(),
                reason: "must be between 0.0 and 1.0".to_string(),
            });
        }

        if !salinity_or_solute_mass_fraction.is_finite()
            || !(0.0..=1.0).contains(&salinity_or_solute_mass_fraction)
        {
            return Err(DomainError::InvalidInvariant {
                field: "salinity_or_solute_mass_fraction".to_string(),
                reason: "must be between 0.0 and 1.0".to_string(),
            });
        }

        let mut total_percentage = 0.0;
        let mut formulas = HashSet::new();

        for comp in &composition {
            total_percentage += comp.percentage();
            if !formulas.insert(comp.formula()) {
                return Err(DomainError::InvalidInvariant {
                    field: "composition".to_string(),
                    reason: format!("duplicate formula '{}'", comp.formula()),
                });
            }
        }

        if total_percentage > 100.0 + ATMOSPHERE_COMPOSITION_MAX_PERCENT_OVERAGE {
            return Err(DomainError::InvalidInvariant {
                field: "composition".to_string(),
                reason: "total percentage exceeds limit".to_string(),
            });
        }

        Ok(Self {
            id,
            planet_id,
            average_depth,
            surface_coverage_fraction,
            salinity_or_solute_mass_fraction,
            composition,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn planet_id(&self) -> Uuid {
        self.planet_id
    }

    pub fn average_depth(&self) -> Length {
        self.average_depth
    }

    pub fn surface_coverage_fraction(&self) -> f64 {
        self.surface_coverage_fraction
    }

    pub fn salinity_or_solute_mass_fraction(&self) -> f64 {
        self.salinity_or_solute_mass_fraction
    }

    pub fn composition(&self) -> &[HydrosphereComponent] {
        &self.composition
    }

    pub fn mean_solvent_properties(&self) -> DomainResult<SolventProperties> {
        let mapped: Vec<(String, f64)> = self
            .composition
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        mean_solvent_properties(&mapped)
    }

    pub fn freezing_point(&self) -> DomainResult<Temperature> {
        let props = self.mean_solvent_properties()?;
        Ok(depressed_freezing_point(
            props.normal_melting_point,
            self.salinity_or_solute_mass_fraction,
            props.cryoscopic_constant,
            DEFAULT_SOLUTE_MOLAR_MASS_KG,
            DEFAULT_VAN_T_HOFF_FACTOR,
        ))
    }

    pub fn boiling_point(&self, surface_pressure: Pressure) -> DomainResult<Temperature> {
        let props = self.mean_solvent_properties()?;
        Ok(dynamic_boiling_point(surface_pressure, &props))
    }

    pub fn matter_state(
        &self,
        temperature: Temperature,
        surface_pressure: Pressure,
    ) -> DomainResult<MatterState> {
        determine_hydrosphere_state(temperature, surface_pressure, self)
    }
}
