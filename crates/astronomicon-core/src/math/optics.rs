use crate::chemistry::optics::GasOpticalProperties;
use crate::math::aerosol::AtmosphericAerosolProperties;
use crate::units::constants::{
    BOLTZMANN_CONSTANT, OPTICAL_REFERENCE_WAVELENGTH, STANDARD_ATMOSPHERE_PRESSURE,
    STP_TEMPERATURE,
};
use crate::units::{Angle, Density, Length, Pressure, Temperature, Wavelength};
use std::f64::consts::PI;

pub fn rayleigh_scattering_cross_section(
    wavelength: Wavelength,
    refractivity_stp: f64,
    king_factor: f64,
) -> f64 {
    let lambda = wavelength.value();
    let delta = refractivity_stp;
    let f_k = if king_factor.is_finite() && king_factor > 0.0 {
        king_factor
    } else {
        1.0
    };

    if lambda <= 0.0 || delta <= 0.0 || !lambda.is_finite() || !delta.is_finite() {
        return 0.0;
    }

    let n = 1.0 + delta;
    let n2 = n * n;
    let n2_minus_1 = n2 - 1.0;

    let n_stp = STANDARD_ATMOSPHERE_PRESSURE / (BOLTZMANN_CONSTANT * STP_TEMPERATURE);
    let num = 8.0 * PI.powi(3) * n2_minus_1.powi(2) * f_k;
    let den = 3.0 * n_stp.powi(2) * lambda.powi(4);

    if den <= 0.0 || !den.is_finite() {
        return 0.0;
    }

    let sigma = num / den;
    if !sigma.is_finite() || sigma <= 0.0 {
        0.0
    } else {
        sigma
    }
}

pub fn molecular_number_density(pressure: Pressure, temperature: Temperature) -> f64 {
    let p = pressure.value();
    let t = temperature.value();

    if p <= 0.0 || t <= 0.0 || !p.is_finite() || !t.is_finite() {
        return 0.0;
    }

    let n = p / (BOLTZMANN_CONSTANT * t);
    if !n.is_finite() || n <= 0.0 {
        0.0
    } else {
        n
    }
}

pub fn rayleigh_scattering_coefficient(
    wavelength: Wavelength,
    refractivity_stp: f64,
    king_factor: f64,
    pressure: Pressure,
    temperature: Temperature,
) -> f64 {
    let sigma = rayleigh_scattering_cross_section(wavelength, refractivity_stp, king_factor);
    let n = molecular_number_density(pressure, temperature);
    let beta = sigma * n;

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn absorption_coefficient(
    gas_optical_properties: &GasOpticalProperties,
    wavelength: Wavelength,
    pressure: Pressure,
    temperature: Temperature,
) -> f64 {
    let sigma = gas_optical_properties.absorption_cross_section_at(wavelength);
    let n = molecular_number_density(pressure, temperature);
    let beta = sigma * n;

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

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

    let rho_n = (6.0 * (f_k - 1.0) / (3.0 + 7.0 * f_k)).clamp(0.0, 0.5);
    let cos_theta = theta.cos();
    let num = (1.0 + rho_n) + (1.0 - rho_n) * cos_theta * cos_theta;
    let den = 4.0 * PI * (2.0 + rho_n) / 3.0;

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

pub fn henyey_greenstein_phase_function(
    scattering_angle: Angle,
    asymmetry_factor: f64,
) -> f64 {
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

pub fn relative_airmass(zenith_angle: Angle) -> f64 {
    let z = zenith_angle.value().abs();
    if !z.is_finite() {
        return 1.0;
    }

    let half_pi = PI / 2.0;
    if z >= half_pi {
        return 40.0;
    }

    let cos_z = z.cos();
    let z_deg = z * (180.0 / PI);
    let diff = (96.07995 - z_deg).max(0.001);
    let denom = cos_z + 0.50572 * diff.powf(-1.6364);

    if denom <= 0.0 || !denom.is_finite() {
        40.0
    } else {
        (1.0 / denom).clamp(1.0, 40.0)
    }
}

pub fn vertical_optical_depth(
    rayleigh_coeff: f64,
    mie_coeff: f64,
    absorption_coeff: f64,
    scale_height: Length,
    aerosol_scale_height: Length,
) -> f64 {
    let b_r = rayleigh_coeff.max(0.0);
    let b_m = mie_coeff.max(0.0);
    let b_a = absorption_coeff.max(0.0);
    let h = scale_height.value().max(0.0);
    let h_aero = aerosol_scale_height.value().max(0.0);

    if !h.is_finite() || h <= 0.0 {
        return 0.0;
    }

    let total_extinction = (b_r + b_a) * h + b_m * h_aero;
    if !total_extinction.is_finite() || total_extinction <= 0.0 {
        0.0
    } else {
        total_extinction
    }
}

pub fn slant_optical_depth(
    vertical_optical_depth: f64,
    zenith_angle: Angle,
) -> f64 {
    if !vertical_optical_depth.is_finite() || vertical_optical_depth <= 0.0 {
        return 0.0;
    }
    let m = relative_airmass(zenith_angle);
    vertical_optical_depth * m
}

pub fn atmospheric_optical_depth(
    wavelength: Wavelength,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    aerosol_properties: &AtmosphericAerosolProperties,
    scale_height: Length,
    aerosol_scale_height: Length,
    zenith_angle: Angle,
) -> f64 {
    let b_r = rayleigh_scattering_coefficient(
        wavelength,
        gas_optical_properties.refractivity_stp(),
        gas_optical_properties.king_factor(),
        pressure,
        temperature,
    );
    let b_m = mie_scattering_coefficient(
        aerosol_properties,
        wavelength,
        Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH),
    );
    let b_a = absorption_coefficient(gas_optical_properties, wavelength, pressure, temperature);

    let tau_0 = vertical_optical_depth(b_r, b_m, b_a, scale_height, aerosol_scale_height);
    slant_optical_depth(tau_0, zenith_angle)
}

pub fn atmospheric_transmittance(optical_depth: f64) -> f64 {
    if !optical_depth.is_finite() || optical_depth < 0.0 {
        return 1.0;
    }
    (-optical_depth).exp().clamp(0.0, 1.0)
}