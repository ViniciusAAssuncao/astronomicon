use crate::chemistry::molar_mass::mean_molar_mass;
use crate::domain::gas_component::GasComponent;
use crate::error::{ DomainError, DomainResult };
use crate::units::constants::{ ATMOSPHERE_COMPOSITION_MAX_PERCENT_OVERAGE, UNIVERSAL_GAS_CONSTANT };
use crate::units::{
    Acceleration,
    Density,
    Length,
    MolarMass,
    Pressure,
    Temperature,
    TemperatureGradient,
};
use serde::{ Deserialize, Serialize };
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atmosphere {
    id: Uuid,
    planet_id: Uuid,
    surface_pressure: Pressure,
    greenhouse_effect: Temperature,
    lapse_rate: TemperatureGradient,
    composition: Vec<GasComponent>,
}

impl Atmosphere {
    pub fn new(
        id: Uuid,
        planet_id: Uuid,
        surface_pressure: Pressure,
        greenhouse_effect: Temperature,
        lapse_rate: TemperatureGradient,
        composition: Vec<GasComponent>
    ) -> DomainResult<Self> {
        if !surface_pressure.value().is_finite() || surface_pressure.value() < 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "surface_pressure".to_string(),
                reason: "must be finite and non-negative".to_string(),
            });
        }

        if !greenhouse_effect.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "greenhouse_effect".to_string(),
                reason: "must be finite".to_string(),
            });
        }

        if !lapse_rate.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "lapse_rate".to_string(),
                reason: "must be finite".to_string(),
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
            surface_pressure,
            greenhouse_effect,
            lapse_rate,
            composition,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn planet_id(&self) -> Uuid {
        self.planet_id
    }

    pub fn surface_pressure(&self) -> Pressure {
        self.surface_pressure
    }

    pub fn greenhouse_effect(&self) -> Temperature {
        self.greenhouse_effect
    }

    pub fn lapse_rate(&self) -> TemperatureGradient {
        self.lapse_rate
    }

    pub fn composition(&self) -> &[GasComponent] {
        &self.composition
    }

    pub fn mean_molar_mass(&self) -> DomainResult<MolarMass> {
        let mapped: Vec<(String, f64)> = self.composition
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        mean_molar_mass(&mapped)
    }

    pub fn density_at_surface(&self, surface_temperature: Temperature) -> DomainResult<Density> {
        if surface_temperature.value() <= 0.0 {
            return Ok(Density::new(0.0));
        }
        let molar_mass = self.mean_molar_mass()?;
        let rho =
            (self.surface_pressure.value() * molar_mass.value()) /
            (UNIVERSAL_GAS_CONSTANT * surface_temperature.value());
        Ok(Density::new(rho))
    }

    pub fn scale_height(
        &self,
        gravity: Acceleration,
        surface_temperature: Temperature
    ) -> DomainResult<Length> {
        let molar_mass = self.mean_molar_mass()?;
        let denom = molar_mass.value() * gravity.value();
        if denom <= 0.0 {
            return Ok(Length::new(0.0));
        }
        let h = (UNIVERSAL_GAS_CONSTANT * surface_temperature.value()) / denom;
        Ok(Length::new(h))
    }

    pub fn pressure_at_altitude(&self, altitude: Length, scale_height: Length) -> Pressure {
        if scale_height.value() <= 0.0 {
            if altitude.value() <= 0.0 {
                return self.surface_pressure;
            }
            return Pressure::new(0.0);
        }
        let exponent = -altitude.value() / scale_height.value();
        Pressure::new(self.surface_pressure.value() * exponent.exp())
    }
}
