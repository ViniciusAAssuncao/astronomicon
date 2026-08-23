use crate::error::{DomainError, DomainResult};
use crate::units::Wavelength;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AbsorptionBand {
    peak_wavelength: Wavelength,
    cross_section_max: f64,
    fwhm: Wavelength,
}

impl AbsorptionBand {
    pub fn new(peak_wavelength: Wavelength, cross_section_max: f64, fwhm: Wavelength) -> Self {
        Self {
            peak_wavelength,
            cross_section_max,
            fwhm,
        }
    }

    pub fn peak_wavelength(&self) -> Wavelength {
        self.peak_wavelength
    }

    pub fn cross_section_max(&self) -> f64 {
        self.cross_section_max
    }

    pub fn fwhm(&self) -> Wavelength {
        self.fwhm
    }

    pub fn cross_section_at(&self, wavelength: Wavelength) -> f64 {
        let lambda = wavelength.value();
        let lambda_0 = self.peak_wavelength.value();
        let fwhm = self.fwhm.value();
        let sigma_max = self.cross_section_max;

        if !lambda.is_finite()
            || !lambda_0.is_finite()
            || !fwhm.is_finite()
            || fwhm <= 0.0
            || !sigma_max.is_finite()
            || sigma_max <= 0.0
        {
            return 0.0;
        }

        let gaussian_sigma = fwhm / (2.0 * (2.0 * std::f64::consts::LN_2).sqrt());
        if gaussian_sigma <= 0.0 || !gaussian_sigma.is_finite() {
            return 0.0;
        }

        let delta = lambda - lambda_0;
        let exponent = -(delta * delta) / (2.0 * gaussian_sigma * gaussian_sigma);

        if exponent < -700.0 {
            return 0.0;
        }

        let value = sigma_max * exponent.exp();
        if !value.is_finite() || value < 0.0 {
            0.0
        } else {
            value
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GasOpticalProperties {
    refractivity_stp: f64,
    king_factor: f64,
    absorption_bands: Vec<AbsorptionBand>,
}

impl GasOpticalProperties {
    pub fn new(
        refractivity_stp: f64,
        king_factor: f64,
        absorption_bands: Vec<AbsorptionBand>,
    ) -> Self {
        Self {
            refractivity_stp,
            king_factor,
            absorption_bands,
        }
    }

    pub fn refractivity_stp(&self) -> f64 {
        self.refractivity_stp
    }

    pub fn king_factor(&self) -> f64 {
        self.king_factor
    }

    pub fn absorption_bands(&self) -> &[AbsorptionBand] {
        &self.absorption_bands
    }

    pub fn refractive_index_stp(&self) -> f64 {
        1.0 + self.refractivity_stp
    }

    pub fn absorption_cross_section_at(&self, wavelength: Wavelength) -> f64 {
        let total: f64 = self
            .absorption_bands
            .iter()
            .map(|band| band.cross_section_at(wavelength))
            .sum();

        if !total.is_finite() || total < 0.0 {
            0.0
        } else {
            total
        }
    }
}

fn o3_absorption_bands() -> Vec<AbsorptionBand> {
    vec![
        AbsorptionBand::new(Wavelength::new(255.0e-9), 1.1e-21, Wavelength::new(40.0e-9)),
        AbsorptionBand::new(Wavelength::new(325.0e-9), 5.0e-23, Wavelength::new(50.0e-9)),
        AbsorptionBand::new(Wavelength::new(600.0e-9), 5.0e-25, Wavelength::new(150.0e-9)),
    ]
}

fn h2o_absorption_bands() -> Vec<AbsorptionBand> {
    vec![
        AbsorptionBand::new(Wavelength::new(942.0e-9), 1.0e-25, Wavelength::new(40.0e-9)),
        AbsorptionBand::new(Wavelength::new(1130.0e-9), 3.0e-25, Wavelength::new(60.0e-9)),
        AbsorptionBand::new(Wavelength::new(1380.0e-9), 2.0e-24, Wavelength::new(90.0e-9)),
        AbsorptionBand::new(Wavelength::new(1870.0e-9), 4.0e-24, Wavelength::new(120.0e-9)),
        AbsorptionBand::new(Wavelength::new(2700.0e-9), 1.5e-23, Wavelength::new(200.0e-9)),
        AbsorptionBand::new(Wavelength::new(6270.0e-9), 3.0e-23, Wavelength::new(1000.0e-9)),
    ]
}

fn co2_absorption_bands() -> Vec<AbsorptionBand> {
    vec![
        AbsorptionBand::new(Wavelength::new(2000.0e-9), 5.0e-25, Wavelength::new(80.0e-9)),
        AbsorptionBand::new(Wavelength::new(2700.0e-9), 5.0e-24, Wavelength::new(150.0e-9)),
        AbsorptionBand::new(Wavelength::new(4300.0e-9), 5.0e-22, Wavelength::new(300.0e-9)),
        AbsorptionBand::new(Wavelength::new(15000.0e-9), 2.0e-22, Wavelength::new(2000.0e-9)),
    ]
}

fn ch4_absorption_bands() -> Vec<AbsorptionBand> {
    vec![
        AbsorptionBand::new(Wavelength::new(1650.0e-9), 1.0e-24, Wavelength::new(60.0e-9)),
        AbsorptionBand::new(Wavelength::new(2300.0e-9), 5.0e-24, Wavelength::new(100.0e-9)),
        AbsorptionBand::new(Wavelength::new(3300.0e-9), 2.0e-22, Wavelength::new(200.0e-9)),
        AbsorptionBand::new(Wavelength::new(7700.0e-9), 8.0e-23, Wavelength::new(400.0e-9)),
    ]
}

pub fn gas_optical_properties(formula: &str) -> Option<GasOpticalProperties> {
    match formula {
        "He" => Some(GasOpticalProperties::new(3.480e-5, 1.000, Vec::new())),
        "Ar" => Some(GasOpticalProperties::new(2.818e-4, 1.000, Vec::new())),
        "H2" => Some(GasOpticalProperties::new(1.386e-4, 1.032, Vec::new())),
        "N2" => Some(GasOpticalProperties::new(2.980e-4, 1.034, Vec::new())),
        "O2" => Some(GasOpticalProperties::new(2.663e-4, 1.096, Vec::new())),
        "H2O" => Some(GasOpticalProperties::new(2.550e-4, 1.001, h2o_absorption_bands())),
        "O3" => Some(GasOpticalProperties::new(5.200e-4, 1.060, o3_absorption_bands())),
        "CO2" => Some(GasOpticalProperties::new(4.494e-4, 1.150, co2_absorption_bands())),
        "CH4" => Some(GasOpticalProperties::new(4.439e-4, 1.000, ch4_absorption_bands())),
        "NH3" => Some(GasOpticalProperties::new(3.760e-4, 1.070, Vec::new())),
        "SO2" => Some(GasOpticalProperties::new(6.860e-4, 1.240, Vec::new())),
        _ => None,
    }
}

pub fn refractivity_stp_of(formula: &str) -> Option<f64> {
    gas_optical_properties(formula).map(|p| p.refractivity_stp())
}

pub fn king_factor_of(formula: &str) -> Option<f64> {
    gas_optical_properties(formula).map(|p| p.king_factor())
}

pub fn absorption_bands_of(formula: &str) -> Option<Vec<AbsorptionBand>> {
    gas_optical_properties(formula).map(|p| p.absorption_bands().to_vec())
}

pub fn absorption_cross_section_of(formula: &str, wavelength: Wavelength) -> Option<f64> {
    gas_optical_properties(formula).map(|p| p.absorption_cross_section_at(wavelength))
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
    let mut combined_bands: Vec<AbsorptionBand> = Vec::new();

    for (formula, fraction) in composition {
        let props = gas_optical_properties(formula).ok_or_else(|| DomainError::InvalidInvariant {
            field: "composition".to_string(),
            reason: format!("unknown gas optical formula '{}'", formula),
        })?;

        let x = fraction / total_fraction;
        king_weighted_sum += x * props.king_factor();

        for band in props.absorption_bands() {
            combined_bands.push(AbsorptionBand::new(
                band.peak_wavelength(),
                band.cross_section_max() * x,
                band.fwhm(),
            ));
        }
    }

    Ok(GasOpticalProperties::new(
        mean_refractivity,
        king_weighted_sum,
        combined_bands,
    ))
}