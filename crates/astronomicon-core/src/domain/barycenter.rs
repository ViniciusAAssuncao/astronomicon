use crate::domain::orbital_elements::OrbitalElements;
use crate::domain::orbital_parent::OrbitalParent;
use crate::error::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarycenterMember {
    Star(Uuid),
    Planet(Uuid),
    Barycenter(Uuid),
}

impl BarycenterMember {
    pub fn id(&self) -> Uuid {
        match *self {
            Self::Star(id) | Self::Planet(id) | Self::Barycenter(id) => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Barycenter {
    id: Uuid,
    star_system_id: Option<Uuid>,
    name: String,
    member_primary: BarycenterMember,
    member_secondary: BarycenterMember,
    internal_orbital_elements: OrbitalElements,
    orbital_parent: OrbitalParent,
    external_orbital_elements: Option<OrbitalElements>,
}

impl Barycenter {
    pub fn new(
        id: Uuid,
        star_system_id: Option<Uuid>,
        name: String,
        member_primary: BarycenterMember,
        member_secondary: BarycenterMember,
        internal_orbital_elements: OrbitalElements,
        orbital_parent: OrbitalParent,
        external_orbital_elements: Option<OrbitalElements>,
    ) -> DomainResult<Self> {
        if name.trim().is_empty() {
            return Err(DomainError::InvalidInvariant {
                field: "name".to_string(),
                reason: "cannot be empty".to_string(),
            });
        }

        if member_primary == member_secondary || member_primary.id() == member_secondary.id() {
            return Err(DomainError::InvalidInvariant {
                field: "members".to_string(),
                reason: "primary and secondary members must be distinct entities".to_string(),
            });
        }

        if member_primary.id() == id || member_secondary.id() == id {
            return Err(DomainError::InvalidInvariant {
                field: "members".to_string(),
                reason: "barycenter cannot be a member of itself".to_string(),
            });
        }

        if orbital_parent == OrbitalParent::Fixed && external_orbital_elements.is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "external_orbital_elements".to_string(),
                reason: "fixed barycenter cannot have external orbital elements".to_string(),
            });
        }

        if orbital_parent != OrbitalParent::Fixed && external_orbital_elements.is_none() {
            return Err(DomainError::InvalidInvariant {
                field: "external_orbital_elements".to_string(),
                reason: "non-fixed orbiting barycenter must have external orbital elements"
                    .to_string(),
            });
        }

        if let OrbitalParent::Barycenter(parent_barycenter_id) = orbital_parent {
            if parent_barycenter_id == id {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: "barycenter cannot have itself as orbital parent".to_string(),
                });
            }
        }

        Ok(Self {
            id,
            star_system_id,
            name,
            member_primary,
            member_secondary,
            internal_orbital_elements,
            orbital_parent,
            external_orbital_elements,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn star_system_id(&self) -> Option<Uuid> {
        self.star_system_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn member_primary(&self) -> BarycenterMember {
        self.member_primary
    }

    pub fn member_secondary(&self) -> BarycenterMember {
        self.member_secondary
    }

    pub fn internal_orbital_elements(&self) -> OrbitalElements {
        self.internal_orbital_elements
    }

    pub fn orbital_parent(&self) -> OrbitalParent {
        self.orbital_parent
    }

    pub fn external_orbital_elements(&self) -> Option<OrbitalElements> {
        self.external_orbital_elements
    }
}
