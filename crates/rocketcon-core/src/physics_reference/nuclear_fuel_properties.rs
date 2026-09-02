use crate::domain::NuclearReactorType;
use astronomicon_core::units::SpecificEnergy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NuclearFuelType {
    Uranium235,
    Plutonium239,
    DeuteriumTritium,
    DeuteriumDeuterium,
}

impl NuclearFuelType {
    pub fn reactor_type(&self) -> NuclearReactorType {
        match self {
            Self::Uranium235 | Self::Plutonium239 => NuclearReactorType::Fission,
            Self::DeuteriumTritium | Self::DeuteriumDeuterium => NuclearReactorType::Fusion,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NuclearFuelProperties {
    pub theoretical_specific_energy: SpecificEnergy,
    pub realistic_burnup_efficiency: f64,
}

impl NuclearFuelProperties {
    pub const fn new(
        theoretical_specific_energy: SpecificEnergy,
        realistic_burnup_efficiency: f64,
    ) -> Self {
        Self {
            theoretical_specific_energy,
            realistic_burnup_efficiency,
        }
    }

    pub fn theoretical_specific_energy(&self) -> SpecificEnergy {
        self.theoretical_specific_energy
    }

    pub fn realistic_burnup_efficiency(&self) -> f64 {
        self.realistic_burnup_efficiency
    }

    pub fn realistic_specific_energy(&self) -> SpecificEnergy {
        SpecificEnergy::new(
            self.theoretical_specific_energy.value() * self.realistic_burnup_efficiency,
        )
    }
}

pub fn nuclear_fuel_properties_of(fuel: NuclearFuelType) -> NuclearFuelProperties {
    match fuel {
        NuclearFuelType::Uranium235 => {
            NuclearFuelProperties::new(SpecificEnergy::new(8.20e13), 0.05)
        }
        NuclearFuelType::Plutonium239 => {
            NuclearFuelProperties::new(SpecificEnergy::new(8.36e13), 0.08)
        }
        NuclearFuelType::DeuteriumTritium => {
            NuclearFuelProperties::new(SpecificEnergy::new(3.37e14), 0.10)
        }
        NuclearFuelType::DeuteriumDeuterium => {
            NuclearFuelProperties::new(SpecificEnergy::new(8.74e13), 0.03)
        }
    }
}