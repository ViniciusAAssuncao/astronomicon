use crate::domain::propellant_kind::PropellantKind;
use crate::error::RocketDomainResult;
use astronomicon_core::chemistry::molar_mass::molar_mass_of;
use astronomicon_core::chemistry::molecular_formula;
use astronomicon_core::domain::validation::{ validate_not_empty, validate_positive_finite };
use astronomicon_core::units::{ Density, MolarMass };
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Propellant {
    id: Uuid,
    name: String,
    kind: PropellantKind,
    chemical_formula: Option<String>,
    density: Density,
    is_cryogenic: bool,
    is_hypergolic: bool,
}

impl Propellant {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        kind: PropellantKind,
        chemical_formula: Option<String>,
        density: Density,
        is_cryogenic: bool,
        is_hypergolic: bool
    ) -> RocketDomainResult<Self> {
        let name = name.into();
        validate_not_empty(&name, "name")?;
        validate_positive_finite(density.value(), "density")?;

        if let Some(ref formula) = chemical_formula {
            molecular_formula::parse(formula)?;
        }

        Ok(Self {
            id,
            name,
            kind,
            chemical_formula,
            density,
            is_cryogenic,
            is_hypergolic,
        })
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> PropellantKind {
        self.kind
    }

    pub fn chemical_formula(&self) -> Option<&str> {
        self.chemical_formula.as_deref()
    }

    pub fn density(&self) -> Density {
        self.density
    }

    pub fn is_cryogenic(&self) -> bool {
        self.is_cryogenic
    }

    pub fn is_hypergolic(&self) -> bool {
        self.is_hypergolic
    }

    pub fn molar_mass(&self) -> RocketDomainResult<Option<MolarMass>> {
        match &self.chemical_formula {
            Some(formula) => {
                let mm = molar_mass_of(formula)?;
                Ok(Some(mm))
            }
            None => Ok(None),
        }
    }
}
