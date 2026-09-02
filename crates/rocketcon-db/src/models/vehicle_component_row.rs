use crate::error::RocketDbError;
use astronomicon_core::units::Vector3;
use rocketcon_core::domain::VehicleComponentEntry;
use rocketcon_core::error::RocketDomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct VehicleComponentRow {
    pub id: String,
    pub vehicle_id: String,
    pub component_id: String,
    pub instance_label: Option<String>,
    pub mount_offset_x_m: f64,
    pub mount_offset_y_m: f64,
    pub mount_offset_z_m: f64,
    pub actuation_axis_x: Option<f64>,
    pub actuation_axis_y: Option<f64>,
    pub actuation_axis_z: Option<f64>,
}

impl TryFrom<VehicleComponentRow> for VehicleComponentEntry {
    type Error = RocketDbError;

    fn try_from(row: VehicleComponentRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let vehicle_id = Uuid::parse_str(&row.vehicle_id)?;
        let component_id = Uuid::parse_str(&row.component_id)?;

        let mount_offset = Vector3::new(
            row.mount_offset_x_m,
            row.mount_offset_y_m,
            row.mount_offset_z_m,
        );

        let actuation_axis = match (
            row.actuation_axis_x,
            row.actuation_axis_y,
            row.actuation_axis_z,
        ) {
            (None, None, None) => None,
            (Some(x), Some(y), Some(z)) => Some(Vector3::new(x, y, z)),
            _ => {
                return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "actuation_axis".to_string(),
                    reason: "all actuation axis components must be present or all must be null".to_string(),
                }));
            }
        };

        let entry = VehicleComponentEntry::new(
            id,
            vehicle_id,
            component_id,
            row.instance_label,
            mount_offset,
            actuation_axis,
        )?;

        Ok(entry)
    }
}
