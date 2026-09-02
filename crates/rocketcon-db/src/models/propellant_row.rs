use crate::error::RocketDbError;
use astronomicon_core::units::Density;
use rocketcon_core::domain::{Propellant, PropellantKind};
use rocketcon_core::error::RocketDomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct PropellantRow {
    pub id: String,
    pub name: String,
    pub propellant_kind: String,
    pub chemical_formula: Option<String>,
    pub density_kg_per_m3: f64,
    pub is_cryogenic: i64,
    pub is_hypergolic: i64,
}

impl TryFrom<PropellantRow> for Propellant {
    type Error = RocketDbError;

    fn try_from(row: PropellantRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let kind = match row.propellant_kind.as_str() {
            "LiquidFuel" => PropellantKind::LiquidFuel,
            "LiquidOxidizer" => PropellantKind::LiquidOxidizer,
            "SolidPropellant" => PropellantKind::SolidPropellant,
            "Monopropellant" => PropellantKind::Monopropellant,
            "NobleGasPropellant" => PropellantKind::NobleGasPropellant,
            "ReactionMass" => PropellantKind::ReactionMass,
            other => {
                return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "propellant_kind".to_string(),
                    reason: format!("unknown propellant kind: {}", other),
                }));
            }
        };

        let propellant = Propellant::new(
            id,
            row.name,
            kind,
            row.chemical_formula,
            Density::new(row.density_kg_per_m3),
            row.is_cryogenic != 0,
            row.is_hypergolic != 0,
        )?;

        Ok(propellant)
    }
}