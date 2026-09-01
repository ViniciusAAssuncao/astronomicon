use crate::error::RocketResult;
use astronomicon_db::SqlitePool;
use rocketcon_core::math::{aggregate_vehicle_assembly, VehicleAssemblyTotals};
use rocketcon_db::repositories::vehicle as vehicle_repository;
use uuid::Uuid;

pub async fn resolve_vehicle_assembly(
    pool: &SqlitePool,
    vehicle_id: Uuid,
) -> RocketResult<VehicleAssemblyTotals> {
    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let totals = aggregate_vehicle_assembly(&components);
    Ok(totals)
}
