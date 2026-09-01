use crate::error::{RocketDbError, RocketDbResult};
use crate::models::{VehicleComponentRow, VehicleRow};
use crate::repositories::component_repository;
use rocketcon_core::domain::{ComponentRecord, Vehicle, VehicleComponentEntry};
use rocketcon_core::error::RocketDomainError;
use sqlx::SqlitePool;
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
        "SELECT id, vehicle_id, component_id, instance_label FROM vehicle_components WHERE vehicle_id = ? ORDER BY id ASC",
    )
    .bind(vehicle_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let entry = VehicleComponentEntry::try_from(row)?;
        let component_record = component_repository::get_by_id(pool, &entry.component_id())
            .await?
            .ok_or_else(|| {
                RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "component_id".to_string(),
                    reason: format!("component '{}' not found", entry.component_id()),
                })
            })?;
        results.push((entry, component_record));
    }

    Ok(results)
}