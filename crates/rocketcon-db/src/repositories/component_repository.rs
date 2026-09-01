use crate::error::RocketDbResult;
use crate::models::ComponentRow;
use rocketcon_core::domain::Component;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, name, component_kind, dry_mass_kg, length_m, diameter_m, power_consumption_w, manufacturer, manufactured_at_unix_seconds, lore_notes FROM components";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> RocketDbResult<Option<Component>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let row = sqlx::query_as::<_, ComponentRow>(&query)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(Component::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> RocketDbResult<Vec<Component>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    let rows = sqlx::query_as::<_, ComponentRow>(&query)
        .fetch_all(pool)
        .await?;

    rows.into_iter().map(Component::try_from).collect()
}
