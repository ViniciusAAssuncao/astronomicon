use crate::error::{RocketDbError, RocketDbResult};
use crate::models::{VehicleComponentRow, VehicleRow};
use crate::repositories::{component_attributes, component_repository};
use rocketcon_core::domain::{ComponentRecord, Vehicle, VehicleComponentEntry};
use rocketcon_core::error::RocketDomainError;
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, name, vehicle_kind, manufacturer, manufactured_at_unix_seconds, lore_notes FROM vehicles";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> RocketDbResult<Option<Vehicle>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let row = sqlx::query_as::<_, VehicleRow>(&query)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(Vehicle::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> RocketDbResult<Vec<Vehicle>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    let rows = sqlx::query_as::<_, VehicleRow>(&query)
        .fetch_all(pool)
        .await?;

    rows.into_iter().map(Vehicle::try_from).collect()
}

pub async fn list_components_for_vehicle(
    pool: &SqlitePool,
    vehicle_id: &Uuid,
) -> RocketDbResult<Vec<(VehicleComponentEntry, ComponentRecord)>> {
    let rows = sqlx::query_as::<_, VehicleComponentRow>(
        "SELECT id, vehicle_id, component_id, stage_index, instance_label, mount_offset_x_m, mount_offset_y_m, mount_offset_z_m, actuation_axis_x, actuation_axis_y, actuation_axis_z FROM vehicle_components WHERE vehicle_id = ? ORDER BY id ASC",
    )
    .bind(vehicle_id.to_string())
    .fetch_all(pool)
    .await?;

    let attribute_rows =
        component_attributes::fetch_attribute_rows_for_vehicle(pool, vehicle_id).await?;
    let empty_attributes = Vec::new();
    let mut results = Vec::with_capacity(rows.len());
    let mut component_records = HashMap::<Uuid, ComponentRecord>::new();
    for row in rows {
        let entry = VehicleComponentEntry::try_from(row)?;
        let component_id = entry.component_id();
        let component_record = if let Some(record) = component_records.get(&component_id) {
            record.clone()
        } else {
            let attributes = attribute_rows
                .get(&component_id.to_string())
                .unwrap_or(&empty_attributes);
            let record = component_repository::get_by_id_with_attribute_rows(
                pool,
                &component_id,
                Some(attributes),
            )
            .await?
            .ok_or_else(|| {
                RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "component_id".to_string(),
                    reason: format!("component '{}' not found", component_id),
                })
            })?;
            component_records.insert(component_id, record.clone());
            record
        };
        results.push((entry, component_record));
    }

    Ok(results)
}
