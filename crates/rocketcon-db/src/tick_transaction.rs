use crate::error::RocketDbResult;
use astronomicon_db::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

pub struct TickTransactionSession {
    pool: SqlitePool,
    gate: Arc<Mutex<()>>,
}

impl TickTransactionSession {
    pub async fn new(source: &SqlitePool) -> RocketDbResult<Self> {
        Ok(Self {
            pool: open_private_pool(source).await?,
            gate: Arc::new(Mutex::new(())),
        })
    }

    pub async fn begin(&self) -> RocketDbResult<TickTransactionPool> {
        let guard = self.gate.clone().lock_owned().await;
        sqlx::query("BEGIN IMMEDIATE").execute(&self.pool).await?;
        Ok(TickTransactionPool {
            pool: self.pool.clone(),
            finished: false,
            close_on_finish: false,
            guard: Some(guard),
        })
    }

    pub async fn close(self) {
        self.pool.close().await;
    }
}

pub struct TickTransactionPool {
    pool: SqlitePool,
    finished: bool,
    close_on_finish: bool,
    guard: Option<OwnedMutexGuard<()>>,
}

impl TickTransactionPool {
    pub async fn begin(source: &SqlitePool) -> RocketDbResult<Self> {
        let pool = open_private_pool(source).await?;
        if let Err(error) = sqlx::query("BEGIN IMMEDIATE").execute(&pool).await {
            pool.close().await;
            return Err(error.into());
        }
        Ok(Self {
            pool,
            finished: false,
            close_on_finish: true,
            guard: None,
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn commit(mut self) -> RocketDbResult<()> {
        sqlx::query("COMMIT").execute(&self.pool).await?;
        self.finished = true;
        self.guard.take();
        if self.close_on_finish {
            self.pool.close().await;
        }
        Ok(())
    }

    pub async fn rollback(mut self) -> RocketDbResult<()> {
        sqlx::query("ROLLBACK").execute(&self.pool).await?;
        self.finished = true;
        self.guard.take();
        if self.close_on_finish {
            self.pool.close().await;
        }
        Ok(())
    }
}

impl Drop for TickTransactionPool {
    fn drop(&mut self) {
        if !self.finished {
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                let pool = self.pool.clone();
                let guard = self.guard.take();
                let close_on_finish = self.close_on_finish;
                runtime.spawn(async move {
                    let _ = sqlx::query("ROLLBACK").execute(&pool).await;
                    drop(guard);
                    if close_on_finish {
                        pool.close().await;
                    }
                });
            }
        }
    }
}

async fn open_private_pool(source: &SqlitePool) -> RocketDbResult<SqlitePool> {
    let options = (*source.connect_options()).clone();
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .connect_with(options)
        .await?)
}