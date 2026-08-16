#[derive(thiserror::Error, Debug, Clone)]
pub enum DomainError {
    #[error("Invariant violated on field '{field}': {reason}")]
    InvalidInvariant { field: String, reason: String },
    #[error("Numerical convergence failed in '{context}': {reason}")]
    NumericalConvergence { context: String, reason: String },
}

pub type DomainResult<T> = Result<T, DomainError>;
