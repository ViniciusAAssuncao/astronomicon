use crate::chemistry::molecular_formula::parse;
use crate::error::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GasComponent {
    formula: String,
    percentage: f64,
}

impl GasComponent {
    pub fn new(formula: String, percentage: f64) -> DomainResult<Self> {
        if !percentage.is_finite() || !(0.0..=100.0).contains(&percentage) {
            return Err(DomainError::InvalidInvariant {
                field: "percentage".to_string(),
                reason: "must be between 0.0 and 100.0".to_string(),
            });
        }

        parse(&formula)?;

        Ok(Self {
            formula,
            percentage,
        })
    }

    pub fn formula(&self) -> &str {
        &self.formula
    }

    pub fn percentage(&self) -> f64 {
        self.percentage
    }
}
