use astronomicon_core::error::DomainError as AstroDomainError;
use astronomicon_db::error::DbError as AstroDbError;
use chronicon_core::error::DomainError as ChroniDomainError;
use chronicon_db::error::DbError as ChroniDbError;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(String),

    #[error("Domain error: {0}")]
    Domain(String),

    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Invalid reference: {0}")]
    InvalidReference(String),
}

impl From<ChroniDbError> for AppError {
    fn from(err: ChroniDbError) -> Self {
        Self::Db(err.to_string())
    }
}

impl From<AstroDbError> for AppError {
    fn from(err: AstroDbError) -> Self {
        Self::Db(err.to_string())
    }
}

impl From<ChroniDomainError> for AppError {
    fn from(err: ChroniDomainError) -> Self {
        Self::Domain(err.to_string())
    }
}

impl From<AstroDomainError> for AppError {
    fn from(err: AstroDomainError) -> Self {
        Self::Domain(err.to_string())
    }
}

impl From<uuid::Error> for AppError {
    fn from(err: uuid::Error) -> Self {
        Self::InvalidReference(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;