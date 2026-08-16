use crate::error::DbResult;
use crate::models::StarSystemRow;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<StarSystemRow>> {
    let row = sqlx
        ::query_as::<_, StarSystemRow>(
            "SELECT id, name, right_ascension_rad, declination_rad, distance_from_sol_m FROM star_systems WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(pool).await?;

    Ok(row)
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<StarSystemRow>> {
    let rows = sqlx
        ::query_as::<_, StarSystemRow>(
            "SELECT id, name, right_ascension_rad, declination_rad, distance_from_sol_m FROM star_systems ORDER BY name ASC"
        )
        .fetch_all(pool).await?;

    Ok(rows)
}
