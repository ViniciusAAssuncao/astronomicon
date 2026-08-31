use crate::error::DbResult;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::str::FromStr;

pub const DATABASE_URL: &str = "sqlite://database/astronomicon.db";

pub async fn open_pool(db_url: &str) -> DbResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true)
        .pragma("journal_mode", "WAL")
        .pragma("synchronous", "NORMAL")
        .pragma("busy_timeout", "5000")
        .pragma("temp_store", "MEMORY")
        .pragma("cache_size", "-20000");

    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    let mut migrator = sqlx::migrate!("../../migrations");
    migrator.set_ignore_missing(true);
    migrator.run(&pool).await?;

    Ok(pool)
}