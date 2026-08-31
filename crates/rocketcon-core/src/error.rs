#[derive(thiserror::Error, Debug, Clone)]
pub enum RocketDomainError {
    #[error("Invariant violated on field '{field}': {reason}")]
    InvalidInvariant { field: String, reason: String },
    #[error("Numerical convergence failed in '{context}': {reason}")]
    NumericalConvergence { context: String, reason: String },
}

pub type RocketDomainResult<T> = Result<T, RocketDomainError>;
