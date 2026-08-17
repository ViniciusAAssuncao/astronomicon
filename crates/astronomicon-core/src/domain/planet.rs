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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Planet {
    id: Uuid,
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
}

impl Planet {
    pub fn new(
        id: Uuid,
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
    ) -> DomainResult<Self> {
        if name.trim().is_empty() {
            return Err(DomainError::InvalidInvariant {
                field: "name".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if !mass.value().is_finite() || mass.value() <= 0.0 {
            return Err(DomainError::InvalidInvariant {
                field: "mass".to_string(),
                reason: "must be positive and finite".to_string(),
            });
        }

        if orbital_parent == OrbitalParent::Fixed && orbital_elements.is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "fixed planet cannot have orbital elements".to_string(),
            });
        }

        if orbital_parent != OrbitalParent::Fixed && orbital_elements.is_none() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "non-fixed orbiting planet must have orbital elements".to_string(),
            });
        }

        if let Some(r) = equatorial_radius {
            if !r.value().is_finite() || r.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "equatorial_radius".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(r) = polar_radius {
            if !r.value().is_finite() || r.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "polar_radius".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(rot) = rotation_period {
            if !rot.value().is_finite() || rot.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "rotation_period".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(ob) = obliquity {
            if !ob.value().is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "obliquity".to_string(),
                    reason: "must be finite".to_string(),
                });
            }
        }

        if let Some(geo) = geometric_albedo {
            if !geo.is_finite() || !(0.0..=1.0).contains(&geo) {
                return Err(DomainError::InvalidInvariant {
                    field: "geometric_albedo".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(bond) = bond_albedo {
            if !bond.is_finite() || !(0.0..=1.0).contains(&bond) {
                return Err(DomainError::InvalidInvariant {
                    field: "bond_albedo".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(ti) = thermal_inertia {
            if !ti.is_finite() || !(0.0..=1.0).contains(&ti) {
                return Err(DomainError::InvalidInvariant {
                    field: "thermal_inertia".to_string(),
                    reason: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(sta) = solstice_true_anomaly {
            if !sta.value().is_finite() {
                return Err(DomainError::InvalidInvariant {
                    field: "solstice_true_anomaly".to_string(),
                    reason: "must be finite".to_string(),
                });
            }
        }

        let solstice_true_anomaly =
            solstice_true_anomaly.map(|angle| Angle::new(angle.value().rem_euclid(TAU)));

        Ok(Self {
            id,
            orbital_parent,
            kind,
            name,
            mass,
            equatorial_radius,
            polar_radius,
            rotation_period,
            obliquity,
            geometric_albedo,
            bond_albedo,
            thermal_inertia,
            solstice_true_anomaly,
            orbital_elements,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
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
}
