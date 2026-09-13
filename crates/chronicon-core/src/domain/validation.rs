use crate::error::{DomainError, DomainResult};

pub fn validate_not_empty(value: &str, field: &str) -> DomainResult<()> {
    if value.trim().is_empty() {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "cannot be empty".to_string(),
        });
    }
    Ok(())
}

pub fn validate_finite(value: f64, field: &str) -> DomainResult<()> {
    if !value.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "must be finite".to_string(),
        });
    }
    Ok(())
}

pub fn validate_positive_finite(value: f64, field: &str) -> DomainResult<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "must be positive and finite".to_string(),
        });
    }
    Ok(())
}

pub fn validate_non_negative_finite(value: f64, field: &str) -> DomainResult<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "must be non-negative and finite".to_string(),
        });
    }
    Ok(())
}