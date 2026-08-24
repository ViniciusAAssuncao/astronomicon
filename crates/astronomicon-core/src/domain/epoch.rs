use crate::error::{DomainError, DomainResult};
use crate::units::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UniverseState {
    elapsed_since_j2000: Duration,
}

impl UniverseState {
    pub fn new(elapsed_since_j2000: Duration) -> DomainResult<Self> {
        if !elapsed_since_j2000.value().is_finite() {
            return Err(DomainError::InvalidInvariant {
                field: "elapsed_since_j2000".to_string(),
                reason: "value must be finite".to_string(),
            });
        }
        Ok(Self {
            elapsed_since_j2000,
        })
    }

    pub fn elapsed_since_j2000(&self) -> Duration {
        self.elapsed_since_j2000
    }
}
