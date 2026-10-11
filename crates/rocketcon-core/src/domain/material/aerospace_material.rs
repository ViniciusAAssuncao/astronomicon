use crate::domain::material::material_class::MaterialClass;
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::{
    validate_non_negative_finite, validate_not_empty, validate_positive_finite,
};
use astronomicon_core::units::{Density, Pressure, Temperature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AerospaceMaterialBuilder {
    id: Uuid,
    name: String,
    material_class: MaterialClass,
    density: Density,
    specific_heat_capacity_j_per_kg_k: f64,
    thermal_conductivity_w_per_m_k: f64,
    thermal_expansion_coefficient_per_k: f64,
    melting_point: Option<Temperature>,
    max_service_temperature: Temperature,
    youngs_modulus: Pressure,
    base_yield_strength: Pressure,
    base_ultimate_tensile_strength: Pressure,
    emissivity: f64,
    solar_absorptivity: f64,
    manufacturer: Option<String>,
    manufactured_at_unix_seconds: Option<i64>,
    lore_notes: Option<String>,
}

impl AerospaceMaterialBuilder {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        material_class: MaterialClass,
        density: Density,
        specific_heat_capacity_j_per_kg_k: f64,
        thermal_conductivity_w_per_m_k: f64,
        thermal_expansion_coefficient_per_k: f64,
        max_service_temperature: Temperature,
        youngs_modulus: Pressure,
        base_yield_strength: Pressure,
        base_ultimate_tensile_strength: Pressure,
        emissivity: f64,
        solar_absorptivity: f64,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            material_class,
            density,
            specific_heat_capacity_j_per_kg_k,
            thermal_conductivity_w_per_m_k,
            thermal_expansion_coefficient_per_k,
            melting_point: None,
            max_service_temperature,
            youngs_modulus,
            base_yield_strength,
            base_ultimate_tensile_strength,
            emissivity,
            solar_absorptivity,
            manufacturer: None,
            manufactured_at_unix_seconds: None,
            lore_notes: None,
        }
    }

    pub fn with_melting_point(mut self, melting_point: impl Into<Option<Temperature>>) -> Self {
        self.melting_point = melting_point.into();
        self
    }

    pub fn with_manufacturer(mut self, manufacturer: impl Into<Option<String>>) -> Self {
        self.manufacturer = manufacturer.into();
        self
    }

    pub fn with_manufactured_at_unix_seconds(
        mut self,
        manufactured_at_unix_seconds: impl Into<Option<i64>>,
    ) -> Self {
        self.manufactured_at_unix_seconds = manufactured_at_unix_seconds.into();
        self
    }

    pub fn with_lore_notes(mut self, lore_notes: impl Into<Option<String>>) -> Self {
        self.lore_notes = lore_notes.into();
        self
    }

    pub fn build(self) -> RocketDomainResult<AerospaceMaterial> {
        validate_not_empty(&self.name, "name")?;
        validate_positive_finite(self.density.value(), "density")?;
        validate_positive_finite(
            self.specific_heat_capacity_j_per_kg_k,
            "specific_heat_capacity_j_per_kg_k",
        )?;
        validate_positive_finite(
            self.thermal_conductivity_w_per_m_k,
            "thermal_conductivity_w_per_m_k",
        )?;
        validate_non_negative_finite(
            self.thermal_expansion_coefficient_per_k,
            "thermal_expansion_coefficient_per_k",
        )?;
        validate_positive_finite(
            self.max_service_temperature.value(),
            "max_service_temperature",
        )?;

        if let Some(mp) = self.melting_point {
            validate_positive_finite(mp.value(), "melting_point")?;
            if mp.value() <= self.max_service_temperature.value() {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "melting_point".to_string(),
                    reason: "melting_point must be greater than max_service_temperature".to_string(),
                });
            }
        }

        validate_positive_finite(self.youngs_modulus.value(), "youngs_modulus")?;
        validate_positive_finite(self.base_yield_strength.value(), "base_yield_strength")?;
        validate_positive_finite(
            self.base_ultimate_tensile_strength.value(),
            "base_ultimate_tensile_strength",
        )?;

        if self.base_yield_strength.value() > self.base_ultimate_tensile_strength.value() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "base_yield_strength".to_string(),
                reason: "base_yield_strength cannot exceed base_ultimate_tensile_strength"
                    .to_string(),
            });
        }

        if !self.emissivity.is_finite() || self.emissivity <= 0.0 || self.emissivity > 1.0 {
            return Err(RocketDomainError::InvalidInvariant {
                field: "emissivity".to_string(),
                reason: "must be in range (0, 1]".to_string(),
            });
        }

        if !self.solar_absorptivity.is_finite()
            || self.solar_absorptivity <= 0.0
            || self.solar_absorptivity > 1.0
        {
            return Err(RocketDomainError::InvalidInvariant {
                field: "solar_absorptivity".to_string(),
                reason: "must be in range (0, 1]".to_string(),
            });
        }

        if let Some(ts) = self.manufactured_at_unix_seconds {
            if ts <= 0 {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "manufactured_at_unix_seconds".to_string(),
                    reason: "must be greater than zero".to_string(),
                });
            }
        }

        if let Some(ref m) = self.manufacturer {
            validate_not_empty(m, "manufacturer")?;
        }

        Ok(AerospaceMaterial {
            id: self.id,
            name: self.name,
            material_class: self.material_class,
            density: self.density,
            specific_heat_capacity_j_per_kg_k: self.specific_heat_capacity_j_per_kg_k,
            thermal_conductivity_w_per_m_k: self.thermal_conductivity_w_per_m_k,
            thermal_expansion_coefficient_per_k: self.thermal_expansion_coefficient_per_k,
            melting_point: self.melting_point,
            max_service_temperature: self.max_service_temperature,
            youngs_modulus: self.youngs_modulus,
            base_yield_strength: self.base_yield_strength,
            base_ultimate_tensile_strength: self.base_ultimate_tensile_strength,
            emissivity: self.emissivity,
            solar_absorptivity: self.solar_absorptivity,
            manufacturer: self.manufacturer,
            manufactured_at_unix_seconds: self.manufactured_at_unix_seconds,
            lore_notes: self.lore_notes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AerospaceMaterial {
    id: Uuid,
    name: String,
    material_class: MaterialClass,
    density: Density,
    specific_heat_capacity_j_per_kg_k: f64,
    thermal_conductivity_w_per_m_k: f64,
    thermal_expansion_coefficient_per_k: f64,
    melting_point: Option<Temperature>,
    max_service_temperature: Temperature,
    youngs_modulus: Pressure,
    base_yield_strength: Pressure,
    base_ultimate_tensile_strength: Pressure,
    emissivity: f64,
    solar_absorptivity: f64,
    manufacturer: Option<String>,
    manufactured_at_unix_seconds: Option<i64>,
    lore_notes: Option<String>,
}

impl AerospaceMaterial {
    pub fn builder(
        id: Uuid,
        name: impl Into<String>,
        material_class: MaterialClass,
        density: Density,
        specific_heat_capacity_j_per_kg_k: f64,
        thermal_conductivity_w_per_m_k: f64,
        thermal_expansion_coefficient_per_k: f64,
        max_service_temperature: Temperature,
        youngs_modulus: Pressure,
        base_yield_strength: Pressure,
        base_ultimate_tensile_strength: Pressure,
        emissivity: f64,
        solar_absorptivity: f64,
    ) -> AerospaceMaterialBuilder {
        AerospaceMaterialBuilder::new(
            id,
            name,
            material_class,
            density,
            specific_heat_capacity_j_per_kg_k,
            thermal_conductivity_w_per_m_k,
            thermal_expansion_coefficient_per_k,
            max_service_temperature,
            youngs_modulus,
            base_yield_strength,
            base_ultimate_tensile_strength,
            emissivity,
            solar_absorptivity,
        )
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn material_class(&self) -> MaterialClass {
        self.material_class
    }

    pub fn density(&self) -> Density {
        self.density
    }

    pub fn specific_heat_capacity_j_per_kg_k(&self) -> f64 {
        self.specific_heat_capacity_j_per_kg_k
    }

    pub fn thermal_conductivity_w_per_m_k(&self) -> f64 {
        self.thermal_conductivity_w_per_m_k
    }

    pub fn thermal_expansion_coefficient_per_k(&self) -> f64 {
        self.thermal_expansion_coefficient_per_k
    }

    pub fn melting_point(&self) -> Option<Temperature> {
        self.melting_point
    }

    pub fn max_service_temperature(&self) -> Temperature {
        self.max_service_temperature
    }

    pub fn youngs_modulus(&self) -> Pressure {
        self.youngs_modulus
    }

    pub fn base_yield_strength(&self) -> Pressure {
        self.base_yield_strength
    }

    pub fn base_ultimate_tensile_strength(&self) -> Pressure {
        self.base_ultimate_tensile_strength
    }

    pub fn emissivity(&self) -> f64 {
        self.emissivity
    }

    pub fn solar_absorptivity(&self) -> f64 {
        self.solar_absorptivity
    }

    pub fn manufacturer(&self) -> Option<&str> {
        self.manufacturer.as_deref()
    }

    pub fn manufactured_at_unix_seconds(&self) -> Option<i64> {
        self.manufactured_at_unix_seconds
    }

    pub fn lore_notes(&self) -> Option<&str> {
        self.lore_notes.as_deref()
    }
}
