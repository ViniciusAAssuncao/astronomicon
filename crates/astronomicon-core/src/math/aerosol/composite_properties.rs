use crate::math::optics::mass_optical_efficiencies;
use crate::units::constants::OPTICAL_REFERENCE_WAVELENGTH;
use crate::units::{Density, Length, Wavelength};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericAerosolProperties {
    dust_density: Density,
    volcanic_density: Density,
    cloud_density: Density,
    total_density: Density,
    asymmetry_factor_g: f64,
    base_extinction_coefficient: f64,
    base_scattering_coefficient: f64,
    angstrom_exponent: f64,
}

impl AtmosphericAerosolProperties {
    pub fn new(
        dust_density: Density,
        volcanic_density: Density,
        cloud_density: Density,
        total_density: Density,
        asymmetry_factor_g: f64,
        base_extinction_coefficient: f64,
        base_scattering_coefficient: f64,
        angstrom_exponent: f64,
    ) -> Self {
        Self {
            dust_density,
            volcanic_density,
            cloud_density,
            total_density,
            asymmetry_factor_g,
            base_extinction_coefficient,
            base_scattering_coefficient,
            angstrom_exponent,
        }
    }

    pub fn dust_density(&self) -> Density {
        self.dust_density
    }

    pub fn volcanic_density(&self) -> Density {
        self.volcanic_density
    }

    pub fn cloud_density(&self) -> Density {
        self.cloud_density
    }

    pub fn total_density(&self) -> Density {
        self.total_density
    }

    pub fn asymmetry_factor_g(&self) -> f64 {
        self.asymmetry_factor_g
    }

    pub fn base_extinction_coefficient(&self) -> f64 {
        self.base_extinction_coefficient
    }

    pub fn base_scattering_coefficient(&self) -> f64 {
        self.base_scattering_coefficient
    }

    pub fn angstrom_exponent(&self) -> f64 {
        self.angstrom_exponent
    }
}

pub fn composite_aerosol_properties_from_materials(
    dust_density: Density,
    volcanic_density: Density,
    cloud_density: Density,
    dust_refractive_index: (f64, f64),
    dust_particle_density: Density,
    volcanic_refractive_index: (f64, f64),
    volcanic_particle_density: Density,
    cloud_refractive_index: (f64, f64),
    cloud_particle_density: Density,
    reference_wavelength: Wavelength,
) -> AtmosphericAerosolProperties {
    let d = dust_density.value().max(0.0);
    let v = volcanic_density.value().max(0.0);
    let c = cloud_density.value().max(0.0);
    let total = d + v + c;

    if total <= 0.0 || !total.is_finite() {
        return AtmosphericAerosolProperties::new(
            dust_density,
            volcanic_density,
            cloud_density,
            Density::new(0.0),
            0.0,
            0.0,
            0.0,
            1.0,
        );
    }

    let r_dust = Length::new(1.0e-6);
    let r_volc = Length::new(5.0e-6);
    let r_cloud = Length::new(10.0e-6);

    let (ke_dust, ks_dust, _, g_dust, alpha_dust) = mass_optical_efficiencies(
        r_dust,
        dust_particle_density,
        dust_refractive_index.0,
        dust_refractive_index.1,
        reference_wavelength,
    );

    let (ke_volc, ks_volc, _, g_volc, alpha_volc) = mass_optical_efficiencies(
        r_volc,
        volcanic_particle_density,
        volcanic_refractive_index.0,
        volcanic_refractive_index.1,
        reference_wavelength,
    );

    let (ke_cloud, ks_cloud, _, g_cloud, alpha_cloud) = mass_optical_efficiencies(
        r_cloud,
        cloud_particle_density,
        cloud_refractive_index.0,
        cloud_refractive_index.1,
        reference_wavelength,
    );

    let scatt_dust = d * ks_dust;
    let scatt_volc = v * ks_volc;
    let scatt_cloud = c * ks_cloud;
    let total_scatt = scatt_dust + scatt_volc + scatt_cloud;

    let ext_dust = d * ke_dust;
    let ext_volc = v * ke_volc;
    let ext_cloud = c * ke_cloud;
    let total_ext = ext_dust + ext_volc + ext_cloud;

    let g_weighted = if total_scatt > 0.0 {
        (scatt_dust * g_dust + scatt_volc * g_volc + scatt_cloud * g_cloud) / total_scatt
    } else {
        0.0
    };

    let alpha_weighted = if total_scatt > 0.0 {
        (scatt_dust * alpha_dust + scatt_volc * alpha_volc + scatt_cloud * alpha_cloud)
            / total_scatt
    } else {
        1.0
    };

    AtmosphericAerosolProperties::new(
        dust_density,
        volcanic_density,
        cloud_density,
        Density::new(total),
        g_weighted.clamp(-0.99, 0.99),
        total_ext,
        total_scatt,
        alpha_weighted.clamp(0.0, 4.0),
    )
}

pub fn composite_aerosol_properties(
    dust_density: Density,
    volcanic_density: Density,
    cloud_density: Density,
) -> AtmosphericAerosolProperties {
    composite_aerosol_properties_from_materials(
        dust_density,
        volcanic_density,
        cloud_density,
        (1.55, 0.005),
        Density::new(2650.0),
        (1.52, 0.015),
        Density::new(2400.0),
        (1.333, 1.0e-8),
        Density::new(1000.0),
        Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH),
    )
}

pub fn mie_scattering_coefficient_at_wavelength(
    base_scattering_coeff: f64,
    wavelength: Wavelength,
    reference_wavelength: Wavelength,
    angstrom_exponent: f64,
) -> f64 {
    let beta_0 = base_scattering_coeff;
    let lambda = wavelength.value();
    let lambda_0 = reference_wavelength.value();
    let alpha = angstrom_exponent;

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