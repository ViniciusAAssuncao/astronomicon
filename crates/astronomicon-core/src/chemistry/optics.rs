use crate::error::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GasOpticalProperties {
    refractivity_stp: f64,
    king_factor: f64,
    base_absorption_cross_section_m2: f64,
}

impl GasOpticalProperties {
    pub fn new(
        refractivity_stp: f64,
        king_factor: f64,
        base_absorption_cross_section_m2: f64,
    ) -> Self {
        Self {
            refractivity_stp,
            king_factor,
            base_absorption_cross_section_m2,
        }
    }

    pub fn refractivity_stp(&self) -> f64 {
        self.refractivity_stp
    }

    pub fn king_factor(&self) -> f64 {
        self.king_factor
    }

    pub fn base_absorption_cross_section_m2(&self) -> f64 {
        self.base_absorption_cross_section_m2
    }

    pub fn refractive_index_stp(&self) -> f64 {
        1.0 + self.refractivity_stp
    }
}

pub fn gas_optical_properties(formula: &str) -> Option<GasOpticalProperties> {
    match formula {
        "He" => Some(GasOpticalProperties::new(3.480e-5, 1.000, 0.0)),
        "Ar" => Some(GasOpticalProperties::new(2.818e-4, 1.000, 0.0)),
        "H2" => Some(GasOpticalProperties::new(1.386e-4, 1.032, 0.0)),
        "N2" => Some(GasOpticalProperties::new(2.980e-4, 1.034, 0.0)),
        "O2" => Some(GasOpticalProperties::new(2.663e-4, 1.096, 0.0)),
        "CO2" => Some(GasOpticalProperties::new(4.494e-4, 1.150, 0.0)),
        "CH4" => Some(GasOpticalProperties::new(4.439e-4, 1.000, 1.0e-29)),
        "NH3" => Some(GasOpticalProperties::new(3.760e-4, 1.070, 0.0)),
        "SO2" => Some(GasOpticalProperties::new(6.860e-4, 1.240, 0.0)),
        _ => None,
    }
}

pub fn refractivity_stp_of(formula: &str) -> Option<f64> {
    gas_optical_properties(formula).map(|p| p.refractivity_stp())
}

pub fn king_factor_of(formula: &str) -> Option<f64> {
    gas_optical_properties(formula).map(|p| p.king_factor())
}

pub fn base_absorption_cross_section_of(formula: &str) -> Option<f64> {
    gas_optical_properties(formula).map(|p| p.base_absorption_cross_section_m2())
}

pub fn lorentz_lorenz_term(refractivity: f64) -> f64 {
    let n = 1.0 + refractivity;
    let n2 = n * n;
    (n2 - 1.0) / (n2 + 2.0)
}

pub fn refractivity_from_lorentz_lorenz_term(lorentz_lorenz_term: f64) -> f64 {
    if lorentz_lorenz_term >= 1.0 / 3.0 {
        return f64::INFINITY;
    }
    let n2 = (1.0 + 2.0 * lorentz_lorenz_term) / (1.0 - lorentz_lorenz_term);
    n2.max(0.0).sqrt() - 1.0
}

pub fn mean_refractivity_lorentz_lorenz(composition: &[(String, f64)]) -> DomainResult<f64> {
    let total_fraction: f64 = composition.iter().map(|(_, f)| f).sum();

    if total_fraction <= 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: "total fraction must be positive".to_string(),
        });
    }

    let mut ll_sum = 0.0;

    for (formula, fraction) in composition {
        let refractivity = refractivity_stp_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: format!("unknown gas optical formula '{}'", formula),
        })?;

        let x = fraction / total_fraction;
        ll_sum += x * lorentz_lorenz_term(refractivity);
    }

    Ok(refractivity_from_lorentz_lorenz_term(ll_sum))
}

pub fn mean_refractivity_gladstone_dale(composition: &[(String, f64)]) -> DomainResult<f64> {
    let total_fraction: f64 = composition.iter().map(|(_, f)| f).sum();

    if total_fraction <= 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: "total fraction must be positive".to_string(),
        });
    }

    let mut refractivity_sum = 0.0;

    for (formula, fraction) in composition {
        let refractivity = refractivity_stp_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: format!("unknown gas optical formula '{}'", formula),
        })?;

        let x = fraction / total_fraction;
        refractivity_sum += x * refractivity;
    }

    Ok(refractivity_sum)
}

pub fn mean_gas_optical_properties(
    composition: &[(String, f64)],
) -> DomainResult<GasOpticalProperties> {
    let total_fraction: f64 = composition.iter().map(|(_, f)| f).sum();

    if total_fraction <= 0.0 {
        return Err(DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: "total fraction must be positive".to_string(),
        });
    }

    let mean_refractivity = mean_refractivity_lorentz_lorenz(composition)?;

    let mut king_weighted_sum = 0.0;
    let mut absorption_weighted_sum = 0.0;

    for (formula, fraction) in composition {
        let props = gas_optical_properties(formula).ok_or_else(|| DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: format!("unknown gas optical formula '{}'", formula),
        })?;

        let x = fraction / total_fraction;
        king_weighted_sum += x * props.king_factor();
        absorption_weighted_sum += x * props.base_absorption_cross_section_m2();
    }

    Ok(GasOpticalProperties::new(
        mean_refractivity,
        king_weighted_sum,
        absorption_weighted_sum,
    ))
}