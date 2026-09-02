use crate::domain::component_kind::ComponentKind;
use crate::error::{ RocketDomainError, RocketDomainResult };
use astronomicon_core::domain::validation::{
    validate_non_negative_finite,
    validate_not_empty,
    validate_positive_finite,
};
use astronomicon_core::units::{ Length, Mass };
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ComponentBuilder {
    id: Uuid,
    name: String,
    kind: ComponentKind,
    dry_mass: Mass,
    length: Length,
    diameter: Length,
    power_consumption_w: f64,
    manufacturer: Option<String>,
    manufactured_at_unix_seconds: Option<i64>,
    lore_notes: Option<String>,
}

impl ComponentBuilder {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        kind: ComponentKind,
        dry_mass: Mass,
        length: Length,
        diameter: Length
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            dry_mass,
            length,
            diameter,
            power_consumption_w: 0.0,
            manufacturer: None,
            manufactured_at_unix_seconds: None,
            lore_notes: None,
        }
    }

    pub fn with_power_consumption_w(mut self, power_consumption_w: f64) -> Self {
        self.power_consumption_w = power_consumption_w;
        self
    }

    pub fn with_manufacturer(mut self, manufacturer: impl Into<Option<String>>) -> Self {
        self.manufacturer = manufacturer.into();
        self
    }

    pub fn with_manufactured_at_unix_seconds(
        mut self,
        manufactured_at_unix_seconds: impl Into<Option<i64>>
    ) -> Self {
        self.manufactured_at_unix_seconds = manufactured_at_unix_seconds.into();
        self
    }

    pub fn with_lore_notes(mut self, lore_notes: impl Into<Option<String>>) -> Self {
        self.lore_notes = lore_notes.into();
        self
    }

    pub fn build(self) -> RocketDomainResult<Component> {
        validate_not_empty(&self.name, "name")?;
        validate_positive_finite(self.dry_mass.value(), "dry_mass")?;
        validate_positive_finite(self.length.value(), "length")?;
        validate_positive_finite(self.diameter.value(), "diameter")?;
        validate_non_negative_finite(self.power_consumption_w, "power_consumption_w")?;

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

        Ok(Component {
            id: self.id,
            name: self.name,
            kind: self.kind,
            dry_mass: self.dry_mass,
            length: self.length,
            diameter: self.diameter,
            power_consumption_w: self.power_consumption_w,
            manufacturer: self.manufacturer,
            manufactured_at_unix_seconds: self.manufactured_at_unix_seconds,
            lore_notes: self.lore_notes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    id: Uuid,
    name: String,
    kind: ComponentKind,
    dry_mass: Mass,
    length: Length,
    diameter: Length,
    power_consumption_w: f64,
    manufacturer: Option<String>,
    manufactured_at_unix_seconds: Option<i64>,
    lore_notes: Option<String>,
}

impl Component {
    pub fn builder(
        id: Uuid,
        name: impl Into<String>,
        kind: ComponentKind,
        dry_mass: Mass,
        length: Length,
        diameter: Length
    ) -> ComponentBuilder {
        ComponentBuilder::new(id, name, kind, dry_mass, length, diameter)
    }

    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        kind: ComponentKind,
        dry_mass: Mass,
        length: Length,
        diameter: Length,
        power_consumption_w: f64,
        manufacturer: Option<String>,
        manufactured_at_unix_seconds: Option<i64>,
        lore_notes: Option<String>
    ) -> RocketDomainResult<Self> {
        Self::builder(id, name, kind, dry_mass, length, diameter)
            .with_power_consumption_w(power_consumption_w)
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

    pub fn kind(&self) -> ComponentKind {
        self.kind
    }

    pub fn dry_mass(&self) -> Mass {
        self.dry_mass
    }

    pub fn length(&self) -> Length {
        self.length
    }

    pub fn diameter(&self) -> Length {
        self.diameter
    }

    pub fn power_consumption_w(&self) -> f64 {
        self.power_consumption_w
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
