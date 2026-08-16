#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("{0}")]
    Generic(String),
    #[error(transparent)]
    Db(#[from] astronomicon_db::error::DbError),
}

pub type AppResult<T> = Result<T, AppError>;