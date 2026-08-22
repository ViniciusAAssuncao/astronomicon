use crate::domain::orbital_elements::OrbitalElements;
use crate::domain::orbital_parent::OrbitalParent;
use crate::error::{DomainError, DomainResult};
use crate::units::{Angle, Duration, Length, Mass};
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanetKind {
    Telluric,
    GasGiant,
    IceGiant,
    DwarfPlanet,
    Chthonian,
    CarbonPlanet,
    IcyBody,
    Exotic,
}

#[derive(Debug, Clone)]
pub struct PlanetBuilder {
    id: Uuid,
    name: String,
    mass: Mass,
    kind: PlanetKind,
    orbital_parent: OrbitalParent,
    star_system_id: Option<Uuid>,
    equatorial_radius: Option<Length>,
    polar_radius: Option<Length>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    geometric_albedo: Option<f64>,
    bond_albedo: Option<f64>,
    thermal_inertia: Option<f64>,
    solstice_true_anomaly: Option<Angle>,
    orbital_elements: Option<OrbitalElements>,
    oblateness_j2: Option<f64>,
    hydrosphere_fraction: Option<f64>,
}

impl PlanetBuilder {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        mass: Mass,
        kind: PlanetKind,
        orbital_parent: OrbitalParent,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            mass,
            kind,
            orbital_parent,
            star_system_id: None,
            equatorial_radius: None,
            polar_radius: None,
            rotation_period: None,
            obliquity: None,
            geometric_albedo: None,
            bond_albedo: None,
            thermal_inertia: None,
            solstice_true_anomaly: None,
            orbital_elements: None,
            oblateness_j2: None,
            hydrosphere_fraction: None,
        }
    }

    pub fn with_star_system_id(mut self, star_system_id: impl Into<Option<Uuid>>) -> Self {
        self.star_system_id = star_system_id.into();
        self
    }

    pub fn with_equatorial_radius(
        mut self,
        equatorial_radius: impl Into<Option<Length>>,
    ) -> Self {
        self.equatorial_radius = equatorial_radius.into();
        self
    }

    pub fn with_polar_radius(mut self, polar_radius: impl Into<Option<Length>>) -> Self {
        self.polar_radius = polar_radius.into();
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

    pub fn with_geometric_albedo(mut self, geometric_albedo: impl Into<Option<f64>>) -> Self {
        self.geometric_albedo = geometric_albedo.into();
        self
    }

    pub fn with_bond_albedo(mut self, bond_albedo: impl Into<Option<f64>>) -> Self {
        self.bond_albedo = bond_albedo.into();
        self
    }

    pub fn with_thermal_inertia(mut self, thermal_inertia: impl Into<Option<f64>>) -> Self {
        self.thermal_inertia = thermal_inertia.into();
        self
    }

    pub fn with_solstice_true_anomaly(
        mut self,
        solstice_true_anomaly: impl Into<Option<Angle>>,
    ) -> Self {
        self.solstice_true_anomaly = solstice_true_anomaly.into();
        self
    }

    pub fn with_orbital_elements(
        mut self,
        orbital_elements: impl Into<Option<OrbitalElements>>,
    ) -> Self {
        self.orbital_elements = orbital_elements.into();
        self
    }

    pub fn with_oblateness_j2(mut self, oblateness_j2: impl Into<Option<f64>>) -> Self {
        self.oblateness_j2 = oblateness_j2.into();
        self
    }

    pub fn with_hydrosphere_fraction(
        mut self,
        hydrosphere_fraction: impl Into<Option<f64>>,
    ) -> Self {
        self.hydrosphere_fraction = hydrosphere_fraction.into();
        self
    }

    pub fn build(self) -> DomainResult<Planet> {
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
                reason: "fixed planet cannot have orbital elements".to_string(),
            });
        }

        if self.orbital_parent != OrbitalParent::Fixed && self.orbital_elements.is_none() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "non-fixed orbiting planet must have orbital elements".to_string(),
            });
        }

        if let Some(r) = self.equatorial_radius {
            if !r.value().is_finite() || r.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "equatorial_radius".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(r) = self.polar_radius {
            if !r.value().is_finite() || r.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "polar_radius".to_string(),
                    reason: "must be positive and finite".to_string(),
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

        if let Some(ti) = self.thermal_inertia {
            if !ti.is_finite() || !(0.0..=1.0).contains(&ti) {
                return Err(DomainError::InvalidInvariant {
                    field: "thermal_inertia".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(sta) = self.solstice_true_anomaly {
            if !sta.value().is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "solstice_true_anomaly".to_string(),
                    reason: "must be finite".to_string(),
                });
            }
        }

        if let Some(j2) = self.oblateness_j2 {
            if !j2.is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "oblateness_j2".to_string(),
                    reason: "must be finite".to_string(),
                });
            }
        }

        if let Some(hf) = self.hydrosphere_fraction {
            if !hf.is_finite() || !(0.0..=1.0).contains(&hf) {
                return Err(DomainError::InvalidInvariant {
                    field: "hydrosphere_fraction".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        let solstice_true_anomaly = self
            .solstice_true_anomaly
            .map(|angle| Angle::new(angle.value().rem_euclid(TAU)));

        Ok(Planet {
            id: self.id,
            star_system_id: self.star_system_id,
            orbital_parent: self.orbital_parent,
            kind: self.kind,
            name: self.name,
            mass: self.mass,
            equatorial_radius: self.equatorial_radius,
            polar_radius: self.polar_radius,
            rotation_period: self.rotation_period,
            obliquity: self.obliquity,
            geometric_albedo: self.geometric_albedo,
            bond_albedo: self.bond_albedo,
            thermal_inertia: self.thermal_inertia,
            solstice_true_anomaly,
            orbital_elements: self.orbital_elements,
            oblateness_j2: self.oblateness_j2,
            hydrosphere_fraction: self.hydrosphere_fraction,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Planet {
    id: Uuid,
    star_system_id: Option<Uuid>,
    orbital_parent: OrbitalParent,
    name: String,
    kind: PlanetKind,
    mass: Mass,
    equatorial_radius: Option<Length>,
    polar_radius: Option<Length>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    geometric_albedo: Option<f64>,
    bond_albedo: Option<f64>,
    thermal_inertia: Option<f64>,
    solstice_true_anomaly: Option<Angle>,
    orbital_elements: Option<OrbitalElements>,
    oblateness_j2: Option<f64>,
    hydrosphere_fraction: Option<f64>,
}

impl Planet {
    pub fn builder(
        id: Uuid,
        name: impl Into<String>,
        mass: Mass,
        kind: PlanetKind,
        orbital_parent: OrbitalParent,
    ) -> PlanetBuilder {
        PlanetBuilder::new(id, name, mass, kind, orbital_parent)
    }

    pub fn new(
        id: Uuid,
        star_system_id: Option<Uuid>,
        orbital_parent: OrbitalParent,
        name: String,
        kind: PlanetKind,
        mass: Mass,
        equatorial_radius: Option<Length>,
        polar_radius: Option<Length>,
        rotation_period: Option<Duration>,
        obliquity: Option<Angle>,
        geometric_albedo: Option<f64>,
        bond_albedo: Option<f64>,
        thermal_inertia: Option<f64>,
        solstice_true_anomaly: Option<Angle>,
        orbital_elements: Option<OrbitalElements>,
        oblateness_j2: Option<f64>,
        hydrosphere_fraction: Option<f64>,
    ) -> DomainResult<Self> {
        Self::builder(id, name, mass, kind, orbital_parent)
            .with_star_system_id(star_system_id)
            .with_equatorial_radius(equatorial_radius)
            .with_polar_radius(polar_radius)
            .with_rotation_period(rotation_period)
            .with_obliquity(obliquity)
            .with_geometric_albedo(geometric_albedo)
            .with_bond_albedo(bond_albedo)
            .with_thermal_inertia(thermal_inertia)
            .with_solstice_true_anomaly(solstice_true_anomaly)
            .with_orbital_elements(orbital_elements)
            .with_oblateness_j2(oblateness_j2)
            .with_hydrosphere_fraction(hydrosphere_fraction)
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

    pub fn kind(&self) -> PlanetKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn equatorial_radius(&self) -> Option<Length> {
        self.equatorial_radius
    }

    pub fn polar_radius(&self) -> Option<Length> {
        self.polar_radius
    }

    pub fn rotation_period(&self) -> Option<Duration> {
        self.rotation_period
    }

    pub fn obliquity(&self) -> Option<Angle> {
        self.obliquity
    }

    pub fn geometric_albedo(&self) -> Option<f64> {
        self.geometric_albedo
    }

    pub fn bond_albedo(&self) -> Option<f64> {
        self.bond_albedo
    }

    pub fn thermal_inertia(&self) -> Option<f64> {
        self.thermal_inertia
    }

    pub fn solstice_true_anomaly(&self) -> Option<Angle> {
        self.solstice_true_anomaly
    }

    pub fn orbital_elements(&self) -> Option<OrbitalElements> {
        self.orbital_elements
    }

    pub fn oblateness_j2(&self) -> Option<f64> {
        self.oblateness_j2
    }

    pub fn hydrosphere_fraction(&self) -> Option<f64> {
        self.hydrosphere_fraction
    }
}