use crate::math::aerosol::AtmosphericAerosolProperties;
use crate::units::{Density, Wavelength};

pub fn mie_scattering_coefficient(
    aerosol_properties: &AtmosphericAerosolProperties,
    wavelength: Wavelength,
    reference_wavelength: Wavelength,
) -> f64 {
    let beta_0 = aerosol_properties.base_scattering_coefficient();
    let lambda = wavelength.value();
    let lambda_0 = reference_wavelength.value();
    let alpha = aerosol_properties.angstrom_exponent();

    if beta_0 <= 0.0
        || lambda <= 0.0
        || lambda_0 <= 0.0
        || !beta_0.is_finite()
        || !lambda.is_finite()
        || !lambda_0.is_finite()
        || !alpha.is_finite()
    {
        return 0.0;
    }

    let ratio = lambda_0 / lambda;
    let beta = beta_0 * ratio.powf(alpha);

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn mie_scattering_coefficient_from_density(
    aerosol_density: Density,
    mass_scattering_efficiency: f64,
    wavelength: Wavelength,
    reference_wavelength: Wavelength,
    angstrom_exponent: f64,
) -> f64 {
    let rho = aerosol_density.value();
    let k_s = mass_scattering_efficiency;
    let lambda = wavelength.value();
    let lambda_0 = reference_wavelength.value();
    let alpha = angstrom_exponent;

    if rho <= 0.0
        || k_s <= 0.0
        || lambda <= 0.0
        || lambda_0 <= 0.0
        || !rho.is_finite()
        || !k_s.is_finite()
        || !lambda.is_finite()
        || !lambda_0.is_finite()
        || !alpha.is_finite()
    {
        return 0.0;
    }

    let beta_0 = rho * k_s;
    let ratio = lambda_0 / lambda;
    let beta = beta_0 * ratio.powf(alpha);

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}
