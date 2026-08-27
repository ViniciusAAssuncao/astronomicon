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
    q_ext: f64,
    particle_radius: Length,
    particle_density: Density,
) -> f64 {
    let r = particle_radius.value();
    let rho = particle_density.value();

    if r <= 0.0 || rho <= 0.0 || !r.is_finite() || !rho.is_finite() || !q_ext.is_finite() {
        0.0
    } else {
        (3.0 * q_ext) / (4.0 * rho * r)
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
    let r = particle_radius.value();
    let nr = refractive_index_real;
    let ni = refractive_index_imag;
    let lambda = wavelength.value();

    if r <= 0.0 || lambda <= 0.0 || nr <= 0.0 || !r.is_finite() || !lambda.is_finite() || !nr.is_finite() || !ni.is_finite() {
        return ParticulateOpticalProperties::new(0.0, 1.0, 0.0);
    }

    let x = (2.0 * PI * r) / lambda;
    
    let (q_ext, q_sca) = if x < 1.0 {
        let nr2 = nr * nr;
        let ni2 = ni * ni;
        let a = nr2 - ni2 - 1.0;
        let b = 2.0 * nr * ni;
        let c = nr2 - ni2 + 2.0;
        let d = 2.0 * nr * ni;
        
        let m2_minus_1_sq = a * a + b * b;
        let m2_plus_2_sq = c * c + d * d;
        
        if m2_plus_2_sq <= 0.0 {
            (0.0, 0.0)
        } else {
            let q_sca = (8.0 / 3.0) * x.powi(4) * (m2_minus_1_sq / m2_plus_2_sq);
            let q_abs = 24.0 * x * (nr * ni) / m2_plus_2_sq;
            (q_sca + q_abs, q_sca)
        }
    } else {
        let rho_star = 2.0 * x * (nr - 1.0).abs();
        let q_ext = if rho_star < 1e-4 {
            0.5 * rho_star * rho_star
        } else {
            2.0 - (4.0 / rho_star) * rho_star.sin() + (4.0 / (rho_star * rho_star)) * (1.0 - rho_star.cos())
        };
        
        let q_abs = particulate_absorption_efficiency(particle_radius, refractive_index_imag, wavelength);
        let q_sca = (q_ext - q_abs).max(0.0);
        
        (q_ext, q_sca)
    };

    let k_ext = particulate_mass_extinction_coefficient(q_ext, particle_radius, particle_density);
    
    let ssa = if q_ext > 0.0 {
        (q_sca / q_ext).clamp(0.0, 1.0)
    } else {
        1.0
    };
    
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