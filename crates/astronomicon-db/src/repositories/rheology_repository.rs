use crate::error::DbResult;
use astronomicon_core::domain::PlanetRheology;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_planet_id(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> DbResult<Option<PlanetRheology>> {
    crate::repositories::lithosphere_repository::get_by_planet_id(pool, planet_id).await
}
