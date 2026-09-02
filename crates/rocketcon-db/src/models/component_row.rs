use crate::error::RocketDbError;
use astronomicon_core::units::{Length, Mass};
use rocketcon_core::domain::{Component, ComponentKind};
use rocketcon_core::error::RocketDomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentRow {
    pub id: String,
    pub name: String,
    pub component_kind: String,
    pub dry_mass_kg: f64,
    pub length_m: f64,
    pub diameter_m: f64,
    pub power_consumption_w: f64,
    pub manufacturer: Option<String>,
    pub manufactured_at_unix_seconds: Option<i64>,
    pub lore_notes: Option<String>,
}

impl TryFrom<ComponentRow> for Component {
    type Error = RocketDbError;

    fn try_from(row: ComponentRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let kind = match row.component_kind.as_str() {
            "Engine" => ComponentKind::Engine,
            "PropellantTank" => ComponentKind::PropellantTank,
            "Battery" => ComponentKind::Battery,
            "SolarPanel" => ComponentKind::SolarPanel,
            "Cpu" => ComponentKind::Cpu,
            "ReactionControlThruster" => ComponentKind::ReactionControlThruster,
            "ReactionWheel" => ComponentKind::ReactionWheel,
            "Rtg" => ComponentKind::Rtg,
            "NuclearReactor" => ComponentKind::NuclearReactor,
            "Radiator" => ComponentKind::Radiator,
            other => {
                return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "component_kind".to_string(),
                    reason: format!("unknown component kind: {}", other),
                }));
            }
        };

        let component = Component::builder(
            id,
            row.name,
            kind,
            Mass::new(row.dry_mass_kg),
            Length::new(row.length_m),
            Length::new(row.diameter_m),
        )
        .with_power_consumption_w(row.power_consumption_w)
        .with_manufacturer(row.manufacturer)
        .with_manufactured_at_unix_seconds(row.manufactured_at_unix_seconds)
        .with_lore_notes(row.lore_notes)
        .build()?;

        Ok(component)
    }
}
