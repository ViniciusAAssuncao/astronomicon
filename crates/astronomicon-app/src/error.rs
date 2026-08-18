#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    Generic(String),
    #[error(transparent)]
    Db(#[from] astronomicon_db::error::DbError),
    #[error(transparent)]
    Domain(#[from] astronomicon_core::error::DomainError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type AppResult<T> = Result<T, AppError>;