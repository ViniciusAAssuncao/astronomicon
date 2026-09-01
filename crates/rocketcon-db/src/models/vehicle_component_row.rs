use crate::error::RocketDbError;
use rocketcon_core::domain::VehicleComponentEntry;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct VehicleComponentRow {
    pub id: String,
    pub vehicle_id: String,
    pub component_id: String,
    pub instance_label: Option<String>,
}

impl TryFrom<VehicleComponentRow> for VehicleComponentEntry {
    type Error = RocketDbError;

    fn try_from(row: VehicleComponentRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let vehicle_id = Uuid::parse_str(&row.vehicle_id)?;
        let component_id = Uuid::parse_str(&row.component_id)?;

        Ok(VehicleComponentEntry::new(
            id,
            vehicle_id,
            component_id,
            row.instance_label,
        ))
    }
}