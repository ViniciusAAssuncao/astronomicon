use rocketcon_core::error::RocketDomainError;
use rocketcon_db::error::RocketDbError;

#[derive(thiserror::Error, Debug)]
pub enum RocketError {
    #[error("{0}")]
    Generic(String),
    #[error(transparent)]
    Domain(#[from] RocketDomainError),
    #[error(transparent)]
    Db(#[from] RocketDbError),
    #[error(transparent)]
    Astronomicon(#[from] astronomicon_app::error::AppError),
    #[error(transparent)]
    AstronomiconDb(#[from] astronomicon_db::error::DbError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Uuid(#[from] uuid::Error),
}

pub type RocketResult<T> = Result<T, RocketError>;
