use crate::chemistry::molecular_formula;
use crate::error::{DomainError, DomainResult};
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

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

pub fn validate_finite_and_non_negative(value: f64, field: &str) -> DomainResult<()> {
    if !value.is_finite() || value < 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "must be finite and non-negative".to_string(),
        });
    }
    Ok(())
}

pub fn validate_unit_interval(value: f64, field: &str) -> DomainResult<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "must be between 0.0 and 1.0".to_string(),
        });
    }
    Ok(())
}

pub fn validate_half_open_unit_interval(value: f64, field: &str) -> DomainResult<()> {
    if !value.is_finite() || value < 0.0 || value >= 1.0 {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "must be in range [0, 1)".to_string(),
        });
    }
    Ok(())
}

pub fn validate_percentage(value: f64, field: &str) -> DomainResult<()> {
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "must be between 0.0 and 100.0".to_string(),
        });
    }
    Ok(())
}

pub fn validate_formula_component(formula: &str, percentage: f64) -> DomainResult<()> {
    validate_percentage(percentage, "percentage")?;
    molecular_formula::parse(formula)?;
    Ok(())
}

pub fn validate_composition<'a, T, K, FPercent, FKey>(
    components: &'a [T],
    get_percentage: FPercent,
    get_key: FKey,
    field: &str,
    key_name: &str,
    max_overage: f64,
) -> DomainResult<()>
where
    K: Eq + Hash + Display,
    FPercent: Fn(&'a T) -> f64,
    FKey: Fn(&'a T) -> K,
{
    let mut total_percentage = 0.0;
    let mut seen = HashSet::new();

    for comp in components {
        total_percentage += get_percentage(comp);
        let key = get_key(comp);
        let key_display = key.to_string();
        if !seen.insert(key) {
            return Err(DomainError::InvalidInvariant {
                field: field.to_string(),
                reason: format!("duplicate {} '{}'", key_name, key_display),
            });
        }
    }

    if total_percentage > 100.0 + max_overage {
        return Err(DomainError::InvalidInvariant {
            field: field.to_string(),
            reason: "total percentage exceeds limit".to_string(),
        });
    }

    Ok(())
}
