use crate::error::AppResult;
use astronomicon_db::SqlitePool;

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
    let db_url = "sqlite://database/astronomicon.db";
    let pool = astronomicon_db::save::initialize_save(db_url).await?;
    Ok(AppContext::new(pool))
}