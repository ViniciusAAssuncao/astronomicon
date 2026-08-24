use crate::units::Angle;
use std::f64::consts::PI;

pub fn rayleigh_phase_function(scattering_angle: Angle) -> f64 {
    let theta = scattering_angle.value();
    if !theta.is_finite() {
        return 1.0 / (4.0 * PI);
    }
    let cos_theta = theta.cos();
    let val = (3.0 / (16.0 * PI)) * (1.0 + cos_theta * cos_theta);
    if !val.is_finite() || val < 0.0 {
        0.0
    } else {
        val
    }
}

pub fn rayleigh_phase_function_with_depolarization(
    scattering_angle: Angle,
    king_factor: f64,
) -> f64 {
    let theta = scattering_angle.value();
    let f_k = if king_factor.is_finite() && king_factor >= 1.0 {
        king_factor
    } else {
        1.0
    };

    if !theta.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let rho_n = ((6.0 * (f_k - 1.0)) / (3.0 + 7.0 * f_k)).clamp(0.0, 0.5);
    let cos_theta = theta.cos();
    let num = 1.0 + rho_n + (1.0 - rho_n) * cos_theta * cos_theta;
    let den = (4.0 * PI * (2.0 + rho_n)) / 3.0;

    if den <= 0.0 || !den.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let val = num / den;
    if !val.is_finite() || val < 0.0 {
        0.0
    } else {
        val
    }
}

pub fn henyey_greenstein_phase_function(scattering_angle: Angle, asymmetry_factor: f64) -> f64 {
    let theta = scattering_angle.value();
    let g = asymmetry_factor.clamp(-0.999, 0.999);

    if !theta.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let cos_theta = theta.cos();
    let denom_base = (1.0 + g * g - 2.0 * g * cos_theta).max(1e-7);
    let denom = 4.0 * PI * denom_base.powf(1.5);

    if denom <= 0.0 || !denom.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let num = 1.0 - g * g;
    let val = num / denom;

    if !val.is_finite() || val < 0.0 {
        0.0
    } else {
        val
    }
}

pub fn combined_scattering_phase_function(
    scattering_angle: Angle,
    rayleigh_coeff: f64,
    mie_coeff: f64,
    asymmetry_factor: f64,
) -> f64 {
    let b_r = rayleigh_coeff.max(0.0);
    let b_m = mie_coeff.max(0.0);
    let total = b_r + b_m;

    if total <= 0.0 || !total.is_finite() {
        return 1.0 / (4.0 * PI);
    }

    let p_r = rayleigh_phase_function(scattering_angle);
    let p_m = henyey_greenstein_phase_function(scattering_angle, asymmetry_factor);

    (b_r * p_r + b_m * p_m) / total
}
