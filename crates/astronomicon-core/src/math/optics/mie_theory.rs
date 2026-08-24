use crate::units::{Density, Length, Wavelength};
use std::f64::consts::PI;

pub fn size_parameter(particle_radius: Length, wavelength: Wavelength) -> f64 {
    let r = particle_radius.value();
    let lambda = wavelength.value();
    if r <= 0.0 || lambda <= 0.0 || !r.is_finite() || !lambda.is_finite() {
        0.0
    } else {
        (2.0 * PI * r) / lambda
    }
}

pub fn van_de_hulst_efficiencies(
    size_parameter: f64,
    refractive_index_real: f64,
    refractive_index_imag: f64,
) -> (f64, f64, f64) {
    let x = size_parameter;
    let nr = refractive_index_real.max(1.0);
    let ni = refractive_index_imag.max(0.0);

    if x <= 0.0 || !x.is_finite() || !nr.is_finite() || !ni.is_finite() {
        return (0.0, 0.0, 0.0);
    }

    let nr_sq = nr * nr;
    let ni_sq = ni * ni;

    let k_denom = (nr_sq - ni_sq + 2.0).powi(2) + 4.0 * nr_sq * ni_sq;
    let (k_real, k_imag) = if k_denom > 0.0 {
        let re = ((nr_sq - ni_sq - 1.0) * (nr_sq - ni_sq + 2.0) + 4.0 * nr_sq * ni_sq) / k_denom;
        let im = (6.0 * nr * ni) / k_denom;
        (re, im)
    } else {
        (0.0, 0.0)
    };

    let k_mag_sq = k_real * k_real + k_imag * k_imag;
    let q_sca_ray = (8.0 / 3.0) * x.powi(4) * k_mag_sq;
    let q_abs_ray = 4.0 * x * k_imag;

    let rho = 2.0 * x * (nr - 1.0);
    let xi = 4.0 * x * ni;
    let tan_beta = if nr > 1.0 { ni / (nr - 1.0) } else { 0.0 };
    let beta = tan_beta.atan();
    let cos_beta = beta.cos();

    let q_ext_vdh = if rho > 1.0e-4 {
        let term1 = 2.0;
        let exp_term = (-rho * tan_beta).exp();
        let cb_over_rho = cos_beta / rho;
        let term2 = 4.0 * exp_term * cb_over_rho * (rho - beta).sin();
        let term3 = 4.0 * exp_term * cb_over_rho * cb_over_rho * (rho - 2.0 * beta).cos();
        let term4 = 4.0 * cb_over_rho * cb_over_rho * (2.0 * beta).cos();
        (term1 - term2 - term3 + term4).max(0.0)
    } else {
        q_sca_ray + q_abs_ray
    };

    let q_abs_vdh = if xi > 1.0e-4 {
        let exp_xi = (-xi).exp();
        (1.0 + (exp_xi * (2.0 * xi + 1.0) - 1.0) / (2.0 * xi * xi)).clamp(0.0, 1.0)
    } else {
        (4.0 / 3.0) * xi * (1.0 - 0.375 * xi).max(0.0)
    };

    let q_sca_vdh = (q_ext_vdh - q_abs_vdh).max(0.0);

    let weight_vdh = (x.powi(4) / (1.0 + x.powi(4))).clamp(0.0, 1.0);
    let q_sca = (1.0 - weight_vdh) * q_sca_ray + weight_vdh * q_sca_vdh;
    let q_abs = (1.0 - weight_vdh) * q_abs_ray + weight_vdh * q_abs_vdh;
    let q_ext = q_sca + q_abs;

    (q_ext, q_sca, q_abs)
}

pub fn mie_asymmetry_factor(
    size_parameter: f64,
    refractive_index_real: f64,
    refractive_index_imag: f64,
) -> f64 {
    let x = size_parameter;
    let nr = refractive_index_real.max(1.0);
    let ni = refractive_index_imag.max(0.0);

    if x <= 0.0 || !x.is_finite() || !nr.is_finite() || !ni.is_finite() {
        return 0.0;
    }

    let size_weight = x * x / (1.0 + x * x);
    let refr_term = (1.0 - 0.45 / nr).clamp(0.0, 1.0);
    let absorp_term = 0.2 * (ni / (ni + 0.1));
    let g = size_weight * (refr_term - absorp_term);

    g.clamp(-0.999, 0.999)
}

pub fn mie_angstrom_exponent(
    particle_radius: Length,
    refractive_index_real: f64,
    refractive_index_imag: f64,
    reference_wavelength: Wavelength,
) -> f64 {
    let r = particle_radius.value();
    let lambda_0 = reference_wavelength.value();

    if r <= 0.0 || lambda_0 <= 0.0 || !r.is_finite() || !lambda_0.is_finite() {
        return 1.0;
    }

    let lambda_1 = Wavelength::new(lambda_0 * 0.9);
    let lambda_2 = Wavelength::new(lambda_0 * 1.1);

    let x1 = size_parameter(particle_radius, lambda_1);
    let x2 = size_parameter(particle_radius, lambda_2);

    let (_, q_sca1, _) =
        van_de_hulst_efficiencies(x1, refractive_index_real, refractive_index_imag);
    let (_, q_sca2, _) =
        van_de_hulst_efficiencies(x2, refractive_index_real, refractive_index_imag);

    if q_sca1 <= 1.0e-12 || q_sca2 <= 1.0e-12 {
        let x0 = size_parameter(particle_radius, reference_wavelength);
        return (4.0 / (1.0 + 0.6 * x0.powf(0.8))).clamp(0.0, 4.0);
    }

    let d_ln_q = (q_sca1 / q_sca2).ln();
    let d_ln_lambda = (lambda_1.value() / lambda_2.value()).ln();

    (-d_ln_q / d_ln_lambda).clamp(0.0, 4.0)
}

pub fn mass_optical_efficiencies(
    particle_radius: Length,
    particle_density: Density,
    refractive_index_real: f64,
    refractive_index_imag: f64,
    wavelength: Wavelength,
) -> (f64, f64, f64, f64, f64) {
    let r = particle_radius.value();
    let rho = particle_density.value();

    if r <= 0.0 || rho <= 0.0 || !r.is_finite() || !rho.is_finite() {
        return (0.0, 0.0, 0.0, 0.0, 1.0);
    }

    let x = size_parameter(particle_radius, wavelength);
    let (q_ext, q_sca, q_abs) =
        van_de_hulst_efficiencies(x, refractive_index_real, refractive_index_imag);
    let g = mie_asymmetry_factor(x, refractive_index_real, refractive_index_imag);
    let alpha = mie_angstrom_exponent(
        particle_radius,
        refractive_index_real,
        refractive_index_imag,
        wavelength,
    );

    let factor = 3.0 / (4.0 * rho * r);
    let k_ext = q_ext * factor;
    let k_sca = q_sca * factor;
    let k_abs = q_abs * factor;

    (k_ext, k_sca, k_abs, g, alpha)
}
