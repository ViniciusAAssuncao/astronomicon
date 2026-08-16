use crate::domain::orbital_elements::OrbitalElements;
use crate::error::{DomainError, DomainResult};
use crate::units::{Angle, Duration, Length, Mass, Temperature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StarKind {
    Star,
    WhiteDwarf,
    NeutronStar,
    BlackHole,
    BrownDwarf,
    Exotic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Star {
    id: Uuid,
    star_system_id: Option<Uuid>,
    kind: StarKind,
    name: String,
    mass: Mass,
    radius: Option<Length>,
    effective_temperature: Option<Temperature>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    orbital_elements: Option<OrbitalElements>,
}

impl Star {
    pub fn new(
        id: Uuid,
        star_system_id: Option<Uuid>,
        kind: StarKind,
        name: String,
        mass: Mass,
        radius: Option<Length>,
        effective_temperature: Option<Temperature>,
        rotation_period: Option<Duration>,
        obliquity: Option<Angle>,
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

        if let Some(r) = radius {
            if !r.value().is_finite() || r.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "radius".to_string(),
                    reason: "must be positive and finite".to_string(),
                });
            }
        }

        if let Some(t) = effective_temperature {
            if !t.value().is_finite() || t.value() <= 0.0 {
                return Err(DomainError::InvalidInvariant {
                    field: "effective_temperature".to_string(),
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

        if star_system_id.is_none() && orbital_elements.is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "cannot have orbital elements without a star system".to_string(),
            });
        }

        Ok(Self {
            id,
            star_system_id,
            kind,
            name,
            mass,
            radius,
            effective_temperature,
            rotation_period,
            obliquity,
            orbital_elements,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn star_system_id(&self) -> Option<Uuid> {
        self.star_system_id
    }

    pub fn kind(&self) -> StarKind {
        self.kind
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn mass(&self) -> Mass {
        self.mass
    }

    pub fn radius(&self) -> Option<Length> {
        self.radius
    }

    pub fn effective_temperature(&self) -> Option<Temperature> {
        self.effective_temperature
    }

    pub fn rotation_period(&self) -> Option<Duration> {
        self.rotation_period
    }

    pub fn obliquity(&self) -> Option<Angle> {
        self.obliquity
    }

    pub fn orbital_elements(&self) -> Option<OrbitalElements> {
        self.orbital_elements
    }
}