use crate::domain::validation::{
    validate_finite_and_non_negative, validate_not_empty, validate_positive_finite,
};
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
        validate_not_empty(&name, "name")?;
        validate_positive_finite(density.value(), "density")?;
        validate_positive_finite(shear_modulus.value(), "shear_modulus")?;
        validate_positive_finite(base_yield_stress.value(), "base_yield_stress")?;
        validate_positive_finite(thermal_conductivity, "thermal_conductivity")?;
        validate_positive_finite(specific_heat_capacity, "specific_heat_capacity")?;
        validate_positive_finite(thermal_expansion, "thermal_expansion")?;
        validate_positive_finite(solidus_temperature.value(), "solidus_temperature")?;

        if !liquidus_temperature.value().is_finite()
            || liquidus_temperature.value() < solidus_temperature.value()
        {
            return Err(DomainError::InvalidInvariant {
                field: "liquidus_temperature".to_string(),
                reason: "must be finite and greater than or equal to solidus temperature"
                    .to_string(),
            });
        }

        if !refractive_index_real.is_finite() || refractive_index_real < 1.0 {
            return Err(DomainError::InvalidInvariant {
                field: "refractive_index_real".to_string(),
                reason: "must be finite and greater than or equal to 1.0".to_string(),
            });
        }

        validate_finite_and_non_negative(refractive_index_imag, "refractive_index_imag")?;

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
