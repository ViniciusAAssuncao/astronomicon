use crate::chemistry::optics::GasOpticalProperties;
use crate::math::aerosol::AtmosphericAerosolProperties;
use crate::math::optics::aerosol_coefficients::mie_scattering_coefficient;
use crate::math::optics::molecular_scattering::{
    absorption_coefficient, rayleigh_scattering_coefficient,
};
use crate::units::constants::OPTICAL_REFERENCE_WAVELENGTH;
use crate::units::{Angle, Length, Pressure, Temperature, Wavelength};
use std::f64::consts::PI;

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

pub fn slant_optical_depth(vertical_optical_depth: f64, zenith_angle: Angle) -> f64 {
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
