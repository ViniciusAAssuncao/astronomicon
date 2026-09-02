pub use crate::domain::decouple_event::DecoupleEvent;
use crate::domain::vehicle_kind::VehicleKind;
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::validate_not_empty;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct VehicleBuilder {
    id: Uuid,
    name: String,
    kind: VehicleKind,
    manufacturer: Option<String>,
    manufactured_at_unix_seconds: Option<i64>,
    lore_notes: Option<String>,
}

impl VehicleBuilder {
    pub fn new(id: Uuid, name: impl Into<String>, kind: VehicleKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            manufacturer: None,
            manufactured_at_unix_seconds: None,
            lore_notes: None,
        }
    }

    pub fn with_manufacturer(mut self, manufacturer: impl Into<Option<String>>) -> Self {
        self.manufacturer = manufacturer.into();
        self
    }

    pub fn with_manufactured_at_unix_seconds(
        mut self,
        manufactured_at_unix_seconds: impl Into<Option<i64>>,
    ) -> Self {
        self.manufactured_at_unix_seconds = manufactured_at_unix_seconds.into();
        self
    }

    pub fn with_lore_notes(mut self, lore_notes: impl Into<Option<String>>) -> Self {
        self.lore_notes = lore_notes.into();
        self
    }

    pub fn build(self) -> RocketDomainResult<Vehicle> {
        validate_not_empty(&self.name, "name")?;

        if let Some(ts) = self.manufactured_at_unix_seconds {
            if ts <= 0 {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "manufactured_at_unix_seconds".to_string(),
                    reason: "must be greater than zero".to_string(),
                });
            }
        }

        if let Some(ref m) = self.manufacturer {
            validate_not_empty(m, "manufacturer")?;
        }

        Ok(Vehicle {
            id: self.id,
            name: self.name,
            kind: self.kind,
            manufacturer: self.manufacturer,
            manufactured_at_unix_seconds: self.manufactured_at_unix_seconds,
            lore_notes: self.lore_notes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vehicle {
    id: Uuid,
    name: String,
    kind: VehicleKind,
    manufacturer: Option<String>,
    manufactured_at_unix_seconds: Option<i64>,
    lore_notes: Option<String>,
}

impl Vehicle {
    pub fn builder(id: Uuid, name: impl Into<String>, kind: VehicleKind) -> VehicleBuilder {
        VehicleBuilder::new(id, name, kind)
    }

    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        kind: VehicleKind,
        manufacturer: Option<String>,
        manufactured_at_unix_seconds: Option<i64>,
        lore_notes: Option<String>,
    ) -> RocketDomainResult<Self> {
        Self::builder(id, name, kind)
            .with_manufacturer(manufacturer)
            .with_manufactured_at_unix_seconds(manufactured_at_unix_seconds)
            .with_lore_notes(lore_notes)
            .build()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> VehicleKind {
        self.kind
    }

    pub fn manufacturer(&self) -> Option<&str> {
        self.manufacturer.as_deref()
    }

    pub fn manufactured_at_unix_seconds(&self) -> Option<i64> {
        self.manufactured_at_unix_seconds
    }

    pub fn lore_notes(&self) -> Option<&str> {
        self.lore_notes.as_deref()
    }
}