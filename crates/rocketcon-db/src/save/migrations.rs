use crate::error::RocketDbResult;
use sqlx::SqlitePool;

pub async fn run_rocketcon_migrations(pool: &SqlitePool) -> RocketDbResult<()> {
    let mut migrator = sqlx::migrate!();
    migrator.set_ignore_missing(true);
    migrator.run(pool).await?;
    Ok(())
}