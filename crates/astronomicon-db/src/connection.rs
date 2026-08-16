use crate::error::DbResult;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};
use std::str::FromStr;

pub async fn open_pool(db_url: &str) -> DbResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str(db_url)?
        .create_if_missing(true)
        .pragma("journal_mode", "WAL")
        .pragma("synchronous", "NORMAL")
        .pragma("busy_timeout", "5000")
        .pragma("temp_store", "MEMORY")
        .pragma("cache_size", "-20000");

    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    Ok(pool)
}
