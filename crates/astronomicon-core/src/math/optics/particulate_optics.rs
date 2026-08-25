use crate::units::{Density, Length, Wavelength};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ParticulateOpticalProperties {
    pub mass_extinction_coefficient: f64,
    pub single_scattering_albedo: f64,
    pub asymmetry_factor: f64,
}

impl ParticulateOpticalProperties {
    pub fn new(
        mass_extinction_coefficient: f64,
        single_scattering_albedo: f64,
        asymmetry_factor: f64,
    ) -> Self {
        Self {
            mass_extinction_coefficient,
            single_scattering_albedo,
            asymmetry_factor,
        }
    }

    pub fn mass_extinction_coefficient(&self) -> f64 {
        self.mass_extinction_coefficient
    }

    pub fn single_scattering_albedo(&self) -> f64 {
        self.single_scattering_albedo
    }

    pub fn asymmetry_factor(&self) -> f64 {
        self.asymmetry_factor
    }
}

pub fn particulate_mass_extinction_coefficient(
    particle_radius: Length,
    particle_density: Density,
) -> f64 {
    let r = particle_radius.value();
    let rho = particle_density.value();

    if r <= 0.0 || rho <= 0.0 || !r.is_finite() || !rho.is_finite() {
        0.0
    } else {
        1.5 / (rho * r)
    }
}

pub fn particulate_absorption_efficiency(
    particle_radius: Length,
    refractive_index_imag: f64,
    wavelength: Wavelength,
) -> f64 {
    let r = particle_radius.value();
    let ni = refractive_index_imag;
    let lambda = wavelength.value();

    if r <= 0.0
        || lambda <= 0.0
        || ni <= 0.0
        || !r.is_finite()
        || !lambda.is_finite()
        || !ni.is_finite()
    {
        return 0.0;
    }

    let xi = (8.0 * PI * r * ni) / lambda;
    if !xi.is_finite() || xi <= 0.0 {
        return 0.0;
    }

    if xi <= 1.0e-4 {
        ((2.0 / 3.0) * xi * (1.0 - 0.375 * xi)).clamp(0.0, 1.0)
    } else if xi >= 700.0 {
        1.0
    } else {
        let exp_xi = (-xi).exp();
        let q_abs = 1.0 + 2.0 * (exp_xi * (xi + 1.0) - 1.0) / (xi * xi);
        q_abs.clamp(0.0, 1.0)
    }
}

pub fn particulate_single_scattering_albedo(
    particle_radius: Length,
    refractive_index_imag: f64,
    wavelength: Wavelength,
) -> f64 {
    let q_abs =
        particulate_absorption_efficiency(particle_radius, refractive_index_imag, wavelength);
    (1.0 - 0.5 * q_abs).clamp(0.0, 1.0)
}

pub fn particulate_asymmetry_factor(
    refractive_index_real: f64,
    refractive_index_imag: f64,
) -> f64 {
    let nr = refractive_index_real;
    let ni = refractive_index_imag;

    if !nr.is_finite() || !ni.is_finite() || nr < 1.0 {
        return 0.0;
    }

    let ni_clamped = ni.max(0.0);
    let refr_term = (1.0 - 0.45 / nr).clamp(0.0, 1.0);
    let absorp_term = 0.2 * (ni_clamped / (ni_clamped + 0.1));
    (refr_term - absorp_term).clamp(-0.999, 0.999)
}

pub fn particulate_optical_properties(
    particle_radius: Length,
    particle_density: Density,
    refractive_index_real: f64,
    refractive_index_imag: f64,
    wavelength: Wavelength,
) -> ParticulateOpticalProperties {
    let k_ext = particulate_mass_extinction_coefficient(particle_radius, particle_density);
    let ssa = particulate_single_scattering_albedo(
        particle_radius,
        refractive_index_imag,
        wavelength,
    );
    let g = particulate_asymmetry_factor(refractive_index_real, refractive_index_imag);

    ParticulateOpticalProperties::new(k_ext, ssa, g)
}

pub fn dust_imaginary_refractive_index(
    iron_mass_fraction: f64,
    wavelength: Wavelength,
) -> f64 {
    let fe = iron_mass_fraction.clamp(0.0, 1.0);
    let lambda = wavelength.value();

    if !lambda.is_finite() || lambda <= 0.0 {
        return 0.001;
    }

    let spectral_scale = (550.0e-9 / lambda).powf(3.5);
    let n_i = 0.0005 + (0.001 + 0.25 * fe) * spectral_scale;
    n_i.clamp(1.0e-5, 0.5)
}

pub fn dust_optical_properties(
    particle_radius: Length,
    particle_density: Density,
    refractive_index_real: f64,
    iron_mass_fraction: f64,
    wavelength: Wavelength,
) -> ParticulateOpticalProperties {
    let n_i = dust_imaginary_refractive_index(iron_mass_fraction, wavelength);
    particulate_optical_properties(
        particle_radius,
        particle_density,
        refractive_index_real,
        n_i,
        wavelength,
    )
}
