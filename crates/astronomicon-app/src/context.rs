use crate::error::AppResult;
use astronomicon_db::SqlitePool;
use astronomicon_db::connection::DATABASE_URL;

pub struct AppContext {
    pool: SqlitePool,
}

impl AppContext {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

pub async fn build_context() -> AppResult<AppContext> {
    let pool = astronomicon_db::save::initialize_save(DATABASE_URL).await?;
    Ok(AppContext::new(pool))
}