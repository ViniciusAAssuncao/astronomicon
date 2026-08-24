use crate::connection::open_pool;
use crate::error::DbResult;
use sqlx::SqlitePool;

pub async fn initialize_save(url: &str) -> DbResult<SqlitePool> {
    open_pool(url).await
}
