use astronomicon_core::error::DomainError;

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Domain(#[from] DomainError),
}

pub type DbResult<T> = Result<T, DbError>;