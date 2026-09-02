use crate::error::RocketDbError;
use rocketcon_core::domain::{Vehicle, VehicleKind};
use rocketcon_core::error::RocketDomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct VehicleRow {
    pub id: String,
    pub name: String,
    pub vehicle_kind: String,
    pub manufacturer: Option<String>,
    pub manufactured_at_unix_seconds: Option<i64>,
    pub lore_notes: Option<String>,
}

impl TryFrom<VehicleRow> for Vehicle {
    type Error = RocketDbError;

    fn try_from(row: VehicleRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let kind = match row.vehicle_kind.as_str() {
            "Rocket" => VehicleKind::Rocket,
            "Spacecraft" => VehicleKind::Spacecraft,
            "Probe" => VehicleKind::Probe,
            "Rover" => VehicleKind::Rover,
            "Satellite" => VehicleKind::Satellite,
            other => {
                return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "vehicle_kind".to_string(),
                    reason: format!("unknown vehicle kind: {}", other),
                }));
            }
        };

        let vehicle = Vehicle::builder(id, row.name, kind)
            .with_manufacturer(row.manufacturer)
            .with_manufactured_at_unix_seconds(row.manufactured_at_unix_seconds)
            .with_lore_notes(row.lore_notes)
            .build()?;

        Ok(vehicle)
    }
}