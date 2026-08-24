use crate::error::{DomainError, DomainResult};
use crate::units::{Density, Pressure, Temperature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialProperties {
    pub id: Uuid,
    pub name: String,
    pub density: Density,
    pub shear_modulus: Pressure,
    pub base_yield_stress: Pressure,
    pub thermal_conductivity: f64,
    pub specific_heat_capacity: f64,
    pub thermal_expansion: f64,
    pub solidus_temperature: Temperature,
    pub liquidus_temperature: Temperature,
    pub refractive_index_real: f64,
    pub refractive_index_imag: f64,
}

impl MaterialProperties {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        density: Density,
        shear_modulus: Pressure,
        base_yield_stress: Pressure,
        thermal_conductivity: f64,
        specific_heat_capacity: f64,
        thermal_expansion: f64,
        solidus_temperature: Temperature,
        liquidus_temperature: Temperature,
        refractive_index_real: f64,
        refractive_index_imag: f64,
    ) -> DomainResult<Self> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(DomainError::InvalidInvariant {
                field: "name".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if !density.value().is_finite() || density.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "density".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !shear_modulus.value().is_finite() || shear_modulus.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "shear_modulus".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !base_yield_stress.value().is_finite() || base_yield_stress.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "base_yield_stress".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !thermal_conductivity.is_finite() || thermal_conductivity <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "thermal_conductivity".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !specific_heat_capacity.is_finite() || specific_heat_capacity <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "specific_heat_capacity".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !thermal_expansion.is_finite() || thermal_expansion <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "thermal_expansion".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !solidus_temperature.value().is_finite() || solidus_temperature.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "solidus_temperature".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if !liquidus_temperature.value().is_finite()
            || liquidus_temperature.value() < solidus_temperature.value()
        {
            return Err(DomainError::InvalidInvariant {
                field: "liquidus_temperature".to_string(),
                reason: "must be finite and greater than or equal to solidus temperature".to_string(),
            });
        }

        if !refractive_index_real.is_finite() || refractive_index_real < 1.0 {
            return Err(DomainError::InvalidInvariant {
                field: "refractive_index_real".to_string(),
                reason: "must be finite and greater than or equal to 1.0".to_string(),
            });
        }

        if !refractive_index_imag.is_finite() || refractive_index_imag < 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "refractive_index_imag".to_string(),
                reason: "must be finite and non-negative".to_string(),
            });
        }

        Ok(Self {
            id,
            name,
            density,
            shear_modulus,
            base_yield_stress,
            thermal_conductivity,
            specific_heat_capacity,
            thermal_expansion,
            solidus_temperature,
            liquidus_temperature,
            refractive_index_real,
            refractive_index_imag,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn density(&self) -> Density {
        self.density
    }

    pub fn shear_modulus(&self) -> Pressure {
        self.shear_modulus
    }

    pub fn base_yield_stress(&self) -> Pressure {
        self.base_yield_stress
    }

    pub fn thermal_conductivity(&self) -> f64 {
        self.thermal_conductivity
    }

    pub fn specific_heat_capacity(&self) -> f64 {
        self.specific_heat_capacity
    }

    pub fn thermal_expansion(&self) -> f64 {
        self.thermal_expansion
    }

    pub fn solidus_temperature(&self) -> Temperature {
        self.solidus_temperature
    }

    pub fn liquidus_temperature(&self) -> Temperature {
        self.liquidus_temperature
    }

    pub fn refractive_index_real(&self) -> f64 {
        self.refractive_index_real
    }

    pub fn refractive_index_imag(&self) -> f64 {
        self.refractive_index_imag
    }
}