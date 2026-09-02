use crate::error::RocketDbResult;
use crate::models::PropellantRow;
use rocketcon_core::domain::Propellant;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, name, propellant_kind, chemical_formula, density_kg_per_m3, is_cryogenic, is_hypergolic FROM propellants";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> RocketDbResult<Option<Propellant>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let row = sqlx::query_as::<_, PropellantRow>(&query)
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(Propellant::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> RocketDbResult<Vec<Propellant>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    let rows = sqlx::query_as::<_, PropellantRow>(&query)
        .fetch_all(pool)
        .await?;

    rows.into_iter().map(Propellant::try_from).collect()
}
