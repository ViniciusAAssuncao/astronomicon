use crate::domain::material::MaterialProperties;
use crate::domain::planet::PlanetKind;
use crate::error::{DomainError, DomainResult};
use crate::units::constants::ATMOSPHERE_COMPOSITION_MAX_PERCENT_OVERAGE;
use crate::units::{Density, Pressure, Temperature};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LithosphereComponent {
    material: MaterialProperties,
    percentage: f64,
}

impl LithosphereComponent {
    pub fn new(material: MaterialProperties, percentage: f64) -> DomainResult<Self> {
        if !percentage.is_finite() || !(0.0..=100.0).contains(&percentage) {
            return Err(DomainError::InvalidInvariant {
                field: "percentage".to_string(),
                reason: "must be between 0.0 and 100.0".to_string(),
            });
        }

        Ok(Self {
            material,
            percentage,
        })
    }

    pub fn material(&self) -> &MaterialProperties {
        &self.material
    }

    pub fn percentage(&self) -> f64 {
        self.percentage
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetRheology {
    components: Vec<LithosphereComponent>,
}

impl PlanetRheology {
    pub fn fallback_for_kind(kind: PlanetKind) -> Self {
        let (name, density, shear, yield_s, k, cp, alpha, t_sol, t_liq, n_r, n_i) = match kind {
            PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet => (
                "Water Ice",
                Density::new(920.0),
                Pressure::new(3.5e9),
                Pressure::new(1.0e6),
                2.2,
                2000.0,
                5.0e-5,
                Temperature::new(273.15),
                Temperature::new(273.15),
                1.31,
                1.0e-8,
            ),
            _ => (
                "Silicate Rock",
                Density::new(3000.0),
                Pressure::new(3.0e10),
                Pressure::new(1.0e8),
                3.0,
                1200.0,
                3.0e-5,
                Temperature::new(1373.15),
                Temperature::new(1673.15),
                1.55,
                0.005,
            ),
        };
        let mat = MaterialProperties::new(
            Uuid::nil(),
            name,
            density,
            shear,
            yield_s,
            k,
            cp,
            alpha,
            t_sol,
            t_liq,
            n_r,
            n_i,
        )
        .expect("valid fallback material");
        let comp = LithosphereComponent::new(mat, 100.0).expect("valid fallback component");
        Self::new(vec![comp]).expect("valid fallback rheology")
    }

    pub fn new(components: Vec<LithosphereComponent>) -> DomainResult<Self> {
        if components.is_empty() {
            return Err(DomainError::InvalidInvariant {
                field: "components".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        let mut total_percentage = 0.0;
        let mut material_ids: HashSet<Uuid> = HashSet::new();

        for comp in &components {
            total_percentage += comp.percentage();
            if !material_ids.insert(comp.material().id()) {
                return Err(DomainError::InvalidInvariant {
                    field: "components".to_string(),
                    reason: format!("duplicate material id '{}'", comp.material().id()),
                });
            }
        }

        if total_percentage > 100.0 + ATMOSPHERE_COMPOSITION_MAX_PERCENT_OVERAGE {
            return Err(DomainError::InvalidInvariant {
                field: "components".to_string(),
                reason: "total percentage exceeds limit".to_string(),
            });
        }

        if total_percentage <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "components".to_string(),
                reason: "total percentage must be positive".to_string(),
            });
        }

        Ok(Self { components })
    }

    pub fn components(&self) -> &[LithosphereComponent] {
        &self.components
    }

    pub fn total_percentage(&self) -> f64 {
        self.components.iter().map(|c| c.percentage()).sum()
    }

    pub fn mean_density(&self) -> Density {
        let total = self.total_percentage();
        let sum: f64 = self
            .components
            .iter()
            .map(|c| c.material().density().value() * (c.percentage() / total))
            .sum();
        Density::new(sum)
    }

    pub fn mean_shear_modulus(&self) -> Pressure {
        let total = self.total_percentage();
        let sum: f64 = self
            .components
            .iter()
            .map(|c| c.material().shear_modulus().value() * (c.percentage() / total))
            .sum();
        Pressure::new(sum)
    }

    pub fn mean_base_yield_stress(&self) -> Pressure {
        let total = self.total_percentage();
        let sum: f64 = self
            .components
            .iter()
            .map(|c| c.material().base_yield_stress().value() * (c.percentage() / total))
            .sum();
        Pressure::new(sum)
    }

    pub fn mean_thermal_conductivity(&self) -> f64 {
        let total = self.total_percentage();
        self.components
            .iter()
            .map(|c| c.material().thermal_conductivity() * (c.percentage() / total))
            .sum()
    }

    pub fn mean_specific_heat_capacity(&self) -> f64 {
        let total = self.total_percentage();
        self.components
            .iter()
            .map(|c| c.material().specific_heat_capacity() * (c.percentage() / total))
            .sum()
    }

    pub fn mean_thermal_expansion(&self) -> f64 {
        let total = self.total_percentage();
        self.components
            .iter()
            .map(|c| c.material().thermal_expansion() * (c.percentage() / total))
            .sum()
    }

    pub fn mean_solidus_temperature(&self) -> Temperature {
        let total = self.total_percentage();
        let sum: f64 = self
            .components
            .iter()
            .map(|c| c.material().solidus_temperature().value() * (c.percentage() / total))
            .sum();
        Temperature::new(sum)
    }

    pub fn mean_liquidus_temperature(&self) -> Temperature {
        let total = self.total_percentage();
        let sum: f64 = self
            .components
            .iter()
            .map(|c| c.material().liquidus_temperature().value() * (c.percentage() / total))
            .sum();
        Temperature::new(sum)
    }

    pub fn mean_refractive_index_real(&self) -> f64 {
        let total = self.total_percentage();
        self.components
            .iter()
            .map(|c| c.material().refractive_index_real() * (c.percentage() / total))
            .sum()
    }

    pub fn mean_refractive_index_imag(&self) -> f64 {
        let total = self.total_percentage();
        self.components
            .iter()
            .map(|c| c.material().refractive_index_imag() * (c.percentage() / total))
            .sum()
    }
}