#[derive(thiserror::Error, Debug, Clone)]
pub enum RocketDomainError {
    #[error("Invariant violated on field '{field}': {reason}")]
    InvalidInvariant { field: String, reason: String },
    #[error("Numerical convergence failed in '{context}': {reason}")]
    NumericalConvergence { context: String, reason: String },
    #[error(transparent)]
    Astronomicon(#[from] astronomicon_core::error::DomainError),
}

pub type RocketDomainResult<T> = Result<T, RocketDomainError>;
