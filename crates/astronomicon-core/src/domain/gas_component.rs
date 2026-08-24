use crate::domain::validation::validate_formula_component;
use crate::error::DomainResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GasComponent {
    formula: String,
    percentage: f64,
}

impl GasComponent {
    pub fn new(formula: String, percentage: f64) -> DomainResult<Self> {
        validate_formula_component(&formula, percentage)?;

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
