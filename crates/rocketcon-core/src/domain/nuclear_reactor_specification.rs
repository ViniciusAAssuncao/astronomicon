use crate::error::{RocketDomainError, RocketDomainResult};
use crate::physics_reference::NuclearFuelType;
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::{Luminosity, Mass};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NuclearReactorType {
    Fission,
    Fusion,
}

impl NuclearReactorType {
    pub fn is_fission(&self) -> bool {
        matches!(self, Self::Fission)
    }

    pub fn is_fusion(&self) -> bool {
        matches!(self, Self::Fusion)
    }
}

#[derive(Debug, Clone)]
pub struct NuclearReactorSpecificationBuilder {
    component_id: Uuid,
    reactor_type: NuclearReactorType,
    fuel_type: NuclearFuelType,
    fuel_mass: Mass,
    max_thermal_power: Luminosity,
    conversion_efficiency: f64,
    min_throttle_fraction: Option<f64>,
}

impl NuclearReactorSpecificationBuilder {
    pub fn new(
        component_id: Uuid,
        reactor_type: NuclearReactorType,
        fuel_type: NuclearFuelType,
        fuel_mass: Mass,
        max_thermal_power: Luminosity,
        conversion_efficiency: f64,
    ) -> Self {
        Self {
            component_id,
            reactor_type,
            fuel_type,
            fuel_mass,
            max_thermal_power,
            conversion_efficiency,
            min_throttle_fraction: None,
        }
    }

    pub fn with_min_throttle_fraction(
        mut self,
        min_throttle_fraction: impl Into<Option<f64>>,
    ) -> Self {
        self.min_throttle_fraction = min_throttle_fraction.into();
        self
    }

    pub fn build(self) -> RocketDomainResult<NuclearReactorSpecification> {
        validate_positive_finite(self.fuel_mass.value(), "fuel_mass")?;
        validate_positive_finite(self.max_thermal_power.value(), "max_thermal_power")?;

        if !self.conversion_efficiency.is_finite()
            || self.conversion_efficiency <= 0.0
            || self.conversion_efficiency > 1.0
        {
            return Err(RocketDomainError::InvalidInvariant {
                field: "conversion_efficiency".to_string(),
                reason: "must be in range (0, 1]".to_string(),
            });
        }

        if self.fuel_type.reactor_type() != self.reactor_type {
            return Err(RocketDomainError::InvalidInvariant {
                field: "fuel_type".to_string(),
                reason: "fuel type is incompatible with reactor type".to_string(),
            });
        }

        if let Some(throttle) = self.min_throttle_fraction {
            validate_positive_finite(throttle, "min_throttle_fraction")?;
            if throttle > 1.0 {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "min_throttle_fraction".to_string(),
                    reason: "must be less than or equal to 1.0".to_string(),
                });
            }
        }

        Ok(NuclearReactorSpecification {
            component_id: self.component_id,
            reactor_type: self.reactor_type,
            fuel_type: self.fuel_type,
            fuel_mass: self.fuel_mass,
            max_thermal_power: self.max_thermal_power,
            conversion_efficiency: self.conversion_efficiency,
            min_throttle_fraction: self.min_throttle_fraction,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NuclearReactorSpecification {
    component_id: Uuid,
    reactor_type: NuclearReactorType,
    fuel_type: NuclearFuelType,
    fuel_mass: Mass,
    max_thermal_power: Luminosity,
    conversion_efficiency: f64,
    min_throttle_fraction: Option<f64>,
}

impl NuclearReactorSpecification {
    pub fn builder(
        component_id: Uuid,
        reactor_type: NuclearReactorType,
        fuel_type: NuclearFuelType,
        fuel_mass: Mass,
        max_thermal_power: Luminosity,
        conversion_efficiency: f64,
    ) -> NuclearReactorSpecificationBuilder {
        NuclearReactorSpecificationBuilder::new(
            component_id,
            reactor_type,
            fuel_type,
            fuel_mass,
            max_thermal_power,
            conversion_efficiency,
        )
    }

    pub fn new(
        component_id: Uuid,
        reactor_type: NuclearReactorType,
        fuel_type: NuclearFuelType,
        fuel_mass: Mass,
        max_thermal_power: Luminosity,
        conversion_efficiency: f64,
        min_throttle_fraction: Option<f64>,
    ) -> RocketDomainResult<Self> {
        Self::builder(
            component_id,
            reactor_type,
            fuel_type,
            fuel_mass,
            max_thermal_power,
            conversion_efficiency,
        )
        .with_min_throttle_fraction(min_throttle_fraction)
        .build()
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn reactor_type(&self) -> NuclearReactorType {
        self.reactor_type
    }

    pub fn fuel_type(&self) -> NuclearFuelType {
        self.fuel_type
    }

    pub fn fuel_mass(&self) -> Mass {
        self.fuel_mass
    }

    pub fn max_thermal_power(&self) -> Luminosity {
        self.max_thermal_power
    }

    pub fn conversion_efficiency(&self) -> f64 {
        self.conversion_efficiency
    }

    pub fn min_throttle_fraction(&self) -> Option<f64> {
        self.min_throttle_fraction
    }

    pub fn is_throttleable(&self) -> bool {
        self.min_throttle_fraction.is_some()
    }

    pub fn max_electric_power(&self) -> Luminosity {
        Luminosity::new(self.max_thermal_power.value() * self.conversion_efficiency)
    }
}