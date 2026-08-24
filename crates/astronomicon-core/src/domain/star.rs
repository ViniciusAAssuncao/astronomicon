use crate::domain::orbital_elements::OrbitalElements;
use crate::domain::orbital_parent::OrbitalParent;
use crate::domain::validation::{validate_finite, validate_not_empty, validate_positive_finite};
use crate::error::{DomainError, DomainResult};
use crate::math::black_hole::{dimensionless_spin_from_rotation_period, event_horizon_radius};
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

#[derive(Debug, Clone)]
pub struct StarBuilder {
    id: Uuid,
    name: String,
    mass: Mass,
    kind: StarKind,
    orbital_parent: OrbitalParent,
    star_system_id: Option<Uuid>,
    radius: Option<Length>,
    effective_temperature: Option<Temperature>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    orbital_elements: Option<OrbitalElements>,
    oblateness_j2: Option<f64>,
    metallicity: Option<f64>,
}

impl StarBuilder {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        mass: Mass,
        kind: StarKind,
        orbital_parent: OrbitalParent,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            mass,
            kind,
            orbital_parent,
            star_system_id: None,
            radius: None,
            effective_temperature: None,
            rotation_period: None,
            obliquity: None,
            orbital_elements: None,
            oblateness_j2: None,
            metallicity: None,
        }
    }

    pub fn with_star_system_id(mut self, star_system_id: impl Into<Option<Uuid>>) -> Self {
        self.star_system_id = star_system_id.into();
        self
    }

    pub fn with_radius(mut self, radius: impl Into<Option<Length>>) -> Self {
        self.radius = radius.into();
        self
    }

    pub fn with_effective_temperature(
        mut self,
        effective_temperature: impl Into<Option<Temperature>>,
    ) -> Self {
        self.effective_temperature = effective_temperature.into();
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

    pub fn with_metallicity(mut self, metallicity: impl Into<Option<f64>>) -> Self {
        self.metallicity = metallicity.into();
        self
    }

    pub fn build(self) -> DomainResult<Star> {
        validate_not_empty(&self.name, "name")?;
        validate_positive_finite(self.mass.value(), "mass")?;

        if self.kind != StarKind::BlackHole {
            if let Some(r) = self.radius {
                validate_positive_finite(r.value(), "radius")?;
            }
        }

        if let Some(t) = self.effective_temperature {
            validate_positive_finite(t.value(), "effective_temperature")?;
        }

        if let Some(rot) = self.rotation_period {
            validate_positive_finite(rot.value(), "rotation_period")?;
        }

        if let Some(ob) = self.obliquity {
            validate_finite(ob.value(), "obliquity")?;
        }

        if let Some(j2) = self.oblateness_j2 {
            validate_finite(j2, "oblateness_j2")?;
        }

        if let Some(met) = self.metallicity {
            validate_finite(met, "metallicity")?;
        }

        if self.orbital_parent == OrbitalParent::Fixed && self.orbital_elements.is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "fixed star cannot have orbital elements".to_string(),
            });
        }

        if self.orbital_parent != OrbitalParent::Fixed && self.orbital_elements.is_none() {
            return Err(DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "non-fixed orbiting star must have orbital elements".to_string(),
            });
        }

        let radius = if self.kind == StarKind::BlackHole {
            let spin = self
                .rotation_period
                .map(|p| dimensionless_spin_from_rotation_period(self.mass, p))
                .unwrap_or(0.0);
            Some(event_horizon_radius(self.mass, spin))
        } else {
            self.radius
        };

        Ok(Star {
            id: self.id,
            star_system_id: self.star_system_id,
            orbital_parent: self.orbital_parent,
            kind: self.kind,
            name: self.name,
            mass: self.mass,
            radius,
            effective_temperature: self.effective_temperature,
            rotation_period: self.rotation_period,
            obliquity: self.obliquity,
            orbital_elements: self.orbital_elements,
            oblateness_j2: self.oblateness_j2,
            metallicity: self.metallicity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Star {
    id: Uuid,
    star_system_id: Option<Uuid>,
    orbital_parent: OrbitalParent,
    kind: StarKind,
    name: String,
    mass: Mass,
    radius: Option<Length>,
    effective_temperature: Option<Temperature>,
    rotation_period: Option<Duration>,
    obliquity: Option<Angle>,
    orbital_elements: Option<OrbitalElements>,
    oblateness_j2: Option<f64>,
    metallicity: Option<f64>,
}

impl Star {
    pub fn builder(
        id: Uuid,
        name: impl Into<String>,
        mass: Mass,
        kind: StarKind,
        orbital_parent: OrbitalParent,
    ) -> StarBuilder {
        StarBuilder::new(id, name, mass, kind, orbital_parent)
    }

    pub fn new(
        id: Uuid,
        star_system_id: Option<Uuid>,
        orbital_parent: OrbitalParent,
        kind: StarKind,
        name: String,
        mass: Mass,
        radius: Option<Length>,
        effective_temperature: Option<Temperature>,
        rotation_period: Option<Duration>,
        obliquity: Option<Angle>,
        orbital_elements: Option<OrbitalElements>,
        oblateness_j2: Option<f64>,
        metallicity: Option<f64>,
    ) -> DomainResult<Self> {
        Self::builder(id, name, mass, kind, orbital_parent)
            .with_star_system_id(star_system_id)
            .with_radius(radius)
            .with_effective_temperature(effective_temperature)
            .with_rotation_period(rotation_period)
            .with_obliquity(obliquity)
            .with_orbital_elements(orbital_elements)
            .with_oblateness_j2(oblateness_j2)
            .with_metallicity(metallicity)
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
        if self.kind == StarKind::BlackHole {
            let spin = self
                .rotation_period
                .map(|p| dimensionless_spin_from_rotation_period(self.mass, p))
                .unwrap_or(0.0);
            Some(event_horizon_radius(self.mass, spin))
        } else {
            self.radius
        }
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

    pub fn oblateness_j2(&self) -> Option<f64> {
        self.oblateness_j2
    }

    pub fn metallicity(&self) -> Option<f64> {
        self.metallicity
    }
}
