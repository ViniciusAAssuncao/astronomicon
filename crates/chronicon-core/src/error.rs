use astronomicon_core::error::DomainError as AstronomiconDomainError;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ChronosError {
    #[error("Invariant violated on field '{field}': {reason}")]
    InvalidInvariant { field: String, reason: String },

    #[error("Numerical convergence failed in '{context}': {reason}")]
    NumericalConvergence { context: String, reason: String },

    #[error(transparent)]
    Astronomicon(#[from] AstronomiconDomainError),
}

pub type ChronosResult<T> = Result<T, ChronosError>;
pub type DomainError = ChronosError;
pub type DomainResult<T> = ChronosResult<T>;