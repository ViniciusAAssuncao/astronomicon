use rocketcon_core::error::RocketDomainError;

#[derive(thiserror::Error, Debug)]
pub enum RocketDbError {
    #[error(transparent)]
    Domain(#[from] RocketDomainError),

    #[error(transparent)]
    Uuid(#[from] uuid::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type RocketDbResult<T> = Result<T, RocketDbError>;