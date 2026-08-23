use crate::domain::orbital_elements::OrbitalElements;
use crate::domain::orbital_parent::OrbitalParent;
use crate::error::{DomainError, DomainResult};
use crate::units::{Angle, Duration, Length, Mass};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpectralType {
    C,
    S,
    M,
    D,
    V,
    P,
}

#[derive(Debug, Clone)]
pub struct MinorPlanetBuilder {
    id: Uuid,
    name: String,
    spectral_type: SpectralType,
    mass: Mass,
    orbital_parent: OrbitalParent,
    star_system_id: Option<Uuid>,
    axis_a: Option<Length>,
    axis_b: Option<Length>,
    axis_c: Option<Length>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    macroporosity: Option<f64>,
    geometric_albedo: Option<f64>,
    bond_albedo: Option<f64>,
    orbital_elements: Option<OrbitalElements>,
}

impl MinorPlanetBuilder {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        spectral_type: SpectralType,
        mass: Mass,
        orbital_parent: OrbitalParent,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            spectral_type,
            mass,
            orbital_parent,
            star_system_id: None,
            axis_a: None,
            axis_b: None,
            axis_c: None,
            rotation_period: None,
            obliquity: None,
            macroporosity: None,
            geometric_albedo: None,
            bond_albedo: None,
            orbital_elements: None,
        }
    }

    pub fn with_star_system_id(mut self, star_system_id: impl Into<Option<Uuid>>) -> Self {
        self.star_system_id = star_system_id.into();
        self
    }

    pub fn with_axis_a(mut self, axis_a: impl Into<Option<Length>>) -> Self {
        self.axis_a = axis_a.into();
        self
    }

    pub fn with_axis_b(mut self, axis_b: impl Into<Option<Length>>) -> Self {
        self.axis_b = axis_b.into();
        self
    }

    pub fn with_axis_c(mut self, axis_c: impl Into<Option<Length>>) -> Self {
        self.axis_c = axis_c.into();
        self
    }

    pub fn with_rotation_period(mut self, rotation_period: impl Into<Option<Duration>>) -> Self {
        self.rotation_period = rotation_period.into();
        self
    }

    pub fn with_obliquity(mut self, obliquity: impl Into<Option<Angle>>) -> Self {
        self.obliquity = obliquity.into();
        self
    }

    pub fn with_macroporosity(mut self, macroporosity: impl Into<Option<f64>>) -> Self {
        self.macroporosity = macroporosity.into();
        self
    }

    pub fn with_geometric_albedo(mut self, geometric_albedo: impl Into<Option<f64>>) -> Self {
        self.geometric_albedo = geometric_albedo.into();
        self
    }

    pub fn with_bond_albedo(mut self, bond_albedo: impl Into<Option<f64>>) -> Self {
        self.bond_albedo = bond_albedo.into();
        self
    }

    pub fn with_orbital_elements(
        mut self,
        orbital_elements: impl Into<Option<OrbitalElements>>,
    ) -> Self {
        self.orbital_elements = orbital_elements.into();
        self
    }

    pub fn build(self) -> DomainResult<MinorPlanet> {
        if self.name.trim().is_empty() {
            return Err(DomainError::InvalidInvariant {
                field: "name".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if !self.mass.value().is_finite() || self.mass.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "mass".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if self.orbital_parent == OrbitalParent::Fixed && self.orbital_elements.is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "fixed minor planet cannot have orbital elements".to_string(),
            });
        }

        if self.orbital_parent != OrbitalParent::Fixed && self.orbital_elements.is_none() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "non-fixed orbiting minor planet must have orbital elements".to_string(),
            });
        }

        if let Some(a) = self.axis_a {
            if !a.value().is_finite() || a.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "axis_a".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(b) = self.axis_b {
            if !b.value().is_finite() || b.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "axis_b".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(c) = self.axis_c {
            if !c.value().is_finite() || c.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "axis_c".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let (Some(a), Some(b)) = (self.axis_a, self.axis_b) {
            if a.value() < b.value() {
                return Err(DomainError::InvalidInvariant {
                    field: "axis_a".to_string(),
                    reason: "axis_a must be greater than or equal to axis_b".to_string(),
                });
            }
        }

        if let (Some(b), Some(c)) = (self.axis_b, self.axis_c) {
            if b.value() < c.value() {
                return Err(DomainError::InvalidInvariant {
                    field: "axis_b".to_string(),
                    reason: "axis_b must be greater than or equal to axis_c".to_string(),
                });
            }
        }

        if let (Some(a), Some(c)) = (self.axis_a, self.axis_c) {
            if a.value() < c.value() {
                return Err(DomainError::InvalidInvariant {
                    field: "axis_a".to_string(),
                    reason: "axis_a must be greater than or equal to axis_c".to_string(),
                });
            }
        }

        if let Some(rot) = self.rotation_period {
            if !rot.value().is_finite() || rot.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "rotation_period".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(ob) = self.obliquity {
            if !ob.value().is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "obliquity".to_string(),
                    reason: "must be finite".to_string(),
                });
            }
        }

        if let Some(mp) = self.macroporosity {
            if !mp.is_finite() || !(0.0..=1.0).contains(&mp) {
                return Err(DomainError::InvalidInvariant {
                    field: "macroporosity".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(geo) = self.geometric_albedo {
            if !geo.is_finite() || !(0.0..=1.0).contains(&geo) {
                return Err(DomainError::InvalidInvariant {
                    field: "geometric_albedo".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(bond) = self.bond_albedo {
            if !bond.is_finite() || !(0.0..=1.0).contains(&bond) {
                return Err(DomainError::InvalidInvariant {
                    field: "bond_albedo".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        Ok(MinorPlanet {
            id: self.id,
            star_system_id: self.star_system_id,
            orbital_parent: self.orbital_parent,
            name: self.name,
            spectral_type: self.spectral_type,
            mass: self.mass,
            axis_a: self.axis_a,
            axis_b: self.axis_b,
            axis_c: self.axis_c,
            rotation_period: self.rotation_period,
            obliquity: self.obliquity,
            macroporosity: self.macroporosity,
            geometric_albedo: self.geometric_albedo,
            bond_albedo: self.bond_albedo,
            orbital_elements: self.orbital_elements,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinorPlanet {
    id: Uuid,
    star_system_id: Option<Uuid>,
    orbital_parent: OrbitalParent,
    name: String,
    spectral_type: SpectralType,
    mass: Mass,
    axis_a: Option<Length>,
    axis_b: Option<Length>,
    axis_c: Option<Length>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    macroporosity: Option<f64>,
    geometric_albedo: Option<f64>,
    bond_albedo: Option<f64>,
    orbital_elements: Option<OrbitalElements>,
}

impl MinorPlanet {
    pub fn builder(
        id: Uuid,
        name: impl Into<String>,
        spectral_type: SpectralType,
        mass: Mass,
        orbital_parent: OrbitalParent,
    ) -> MinorPlanetBuilder {
        MinorPlanetBuilder::new(id, name, spectral_type, mass, orbital_parent)
    }

    pub fn new(
        id: Uuid,
        star_system_id: Option<Uuid>,
        orbital_parent: OrbitalParent,
        name: String,
        spectral_type: SpectralType,
        mass: Mass,
        axis_a: Option<Length>,
        axis_b: Option<Length>,
        axis_c: Option<Length>,
        rotation_period: Option<Duration>,
        obliquity: Option<Angle>,
        macroporosity: Option<f64>,
        geometric_albedo: Option<f64>,
        bond_albedo: Option<f64>,
        orbital_elements: Option<OrbitalElements>,
    ) -> DomainResult<Self> {
        Self::builder(id, name, spectral_type, mass, orbital_parent)
            .with_star_system_id(star_system_id)
            .with_axis_a(axis_a)
            .with_axis_b(axis_b)
            .with_axis_c(axis_c)
            .with_rotation_period(rotation_period)
            .with_obliquity(obliquity)
            .with_macroporosity(macroporosity)
            .with_geometric_albedo(geometric_albedo)
            .with_bond_albedo(bond_albedo)
            .with_orbital_elements(orbital_elements)
            .build()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn star_system_id(&self) -> Option<Uuid> {
        self.star_system_id
    }

    pub fn orbital_parent(&self) -> OrbitalParent {
        self.orbital_parent
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn spectral_type(&self) -> SpectralType {
        self.spectral_type
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn axis_a(&self) -> Option<Length> {
        self.axis_a
    }

    pub fn axis_b(&self) -> Option<Length> {
        self.axis_b
    }

    pub fn axis_c(&self) -> Option<Length> {
        self.axis_c
    }

    pub fn rotation_period(&self) -> Option<Duration> {
        self.rotation_period
    }

    pub fn obliquity(&self) -> Option<Angle> {
        self.obliquity
    }

    pub fn macroporosity(&self) -> Option<f64> {
        self.macroporosity
    }

    pub fn geometric_albedo(&self) -> Option<f64> {
        self.geometric_albedo
    }

    pub fn bond_albedo(&self) -> Option<f64> {
        self.bond_albedo
    }

    pub fn orbital_elements(&self) -> Option<OrbitalElements> {
        self.orbital_elements
    }
}