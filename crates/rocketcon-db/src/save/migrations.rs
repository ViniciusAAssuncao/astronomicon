use crate::error::RocketDbResult;
use sqlx::SqlitePool;

pub async fn run_rocketcon_migrations(pool: &SqlitePool) -> RocketDbResult<()> {
    sqlx::migrate!().run(pool).await?;
    Ok(())
}