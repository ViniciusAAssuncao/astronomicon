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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use std::error::Error;
    use uuid::Uuid;

    #[tokio::test]
    async fn transaction_survives_pool_checkouts_and_rolls_back() -> Result<(), Box<dyn Error>> {
        let path = std::env::temp_dir().join(format!("rocketcon-tick-{}.db", Uuid::new_v4()));
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let source = SqlitePoolOptions::new().connect_with(options).await?;
        sqlx::query("CREATE TABLE values_for_test (value INTEGER NOT NULL)")
            .execute(&source)
            .await?;
        sqlx::query("INSERT INTO values_for_test (value) VALUES (1)")
            .execute(&source)
            .await?;

        let transaction = TickTransactionPool::begin(&source).await?;
        sqlx::query("UPDATE values_for_test SET value = 2")
            .execute(transaction.pool())
            .await?;
        let inside: i64 = sqlx::query_scalar("SELECT value FROM values_for_test")
            .fetch_one(transaction.pool())
            .await?;
        let outside: i64 = sqlx::query_scalar("SELECT value FROM values_for_test")
            .fetch_one(&source)
            .await?;
        assert_eq!((inside, outside), (2, 1));
        transaction.rollback().await?;
        let after_rollback: i64 = sqlx::query_scalar("SELECT value FROM values_for_test")
            .fetch_one(&source)
            .await?;
        assert_eq!(after_rollback, 1);

        let transaction = TickTransactionPool::begin(&source).await?;
        sqlx::query("UPDATE values_for_test SET value = 3")
            .execute(transaction.pool())
            .await?;
        transaction.commit().await?;
        let after_commit: i64 = sqlx::query_scalar("SELECT value FROM values_for_test")
            .fetch_one(&source)
            .await?;
        assert_eq!(after_commit, 3);

        let abandoned = TickTransactionPool::begin(&source).await?;
        sqlx::query("UPDATE values_for_test SET value = 4")
            .execute(abandoned.pool())
            .await?;
        drop(abandoned);
        let next = TickTransactionPool::begin(&source).await?;
        let after_abandonment: i64 = sqlx::query_scalar("SELECT value FROM values_for_test")
            .fetch_one(next.pool())
            .await?;
        assert_eq!(after_abandonment, 3);
        next.rollback().await?;

        let session = TickTransactionSession::new(&source).await?;
        let increment = async {
            let transaction = session.begin().await?;
            sqlx::query("UPDATE values_for_test SET value = value + 1")
                .execute(transaction.pool())
                .await?;
            transaction.commit().await
        };
        let other_increment = async {
            let transaction = session.begin().await?;
            sqlx::query("UPDATE values_for_test SET value = value + 1")
                .execute(transaction.pool())
                .await?;
            transaction.commit().await
        };
        let (first, second) = tokio::join!(increment, other_increment);
        first?;
        second?;
        session.close().await;
        let after_session: i64 = sqlx::query_scalar("SELECT value FROM values_for_test")
            .fetch_one(&source)
            .await?;
        assert_eq!(after_session, 5);

        source.close().await;
        drop(source);
        for attempt in 0..10 {
            match std::fs::remove_file(&path) {
                Ok(()) => break,
                Err(error) if attempt < 9 => {
                    let _ = error;
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
