use crate::chemistry::optics::{AbsorptionBand, GasOpticalProperties};
use crate::math::optics::molecular_scattering::{
    absorption_coefficient, rayleigh_scattering_coefficient,
};
use crate::math::optics::optical_depth::{
    atmospheric_transmittance, slant_optical_depth, vertical_optical_depth,
};
use crate::math::radiation::blackbody::planck_spectral_radiance;
use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{Angle, Irradiance, Length, Pressure, Temperature, Wavelength};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const DNA_DAMAGE_PEAK_WAVELENGTH_M: f64 = 265.0e-9;
pub const DNA_DAMAGE_FWHM_M: f64 = 35.0e-9;
pub const UV_WAVELENGTH_MIN_M: f64 = 200.0e-9;
pub const UV_WAVELENGTH_MAX_M: f64 = 400.0e-9;
pub const UVC_WAVELENGTH_MAX_M: f64 = 280.0e-9;
pub const UVB_WAVELENGTH_MAX_M: f64 = 315.0e-9;

pub fn dna_action_spectrum() -> AbsorptionBand {
    AbsorptionBand::new(
        Wavelength::new(DNA_DAMAGE_PEAK_WAVELENGTH_M),
        1.0,
        Wavelength::new(DNA_DAMAGE_FWHM_M),
    )
}

pub fn dna_damage_spectral_weight(wavelength: Wavelength) -> f64 {
    dna_action_spectrum().cross_section_at(wavelength)
}

pub fn dna_weighted_uv_transmittance(
    star_temperature: Temperature,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    scale_height: Length,
    aerosol_scale_height: Length,
    mie_coeff: f64,
    zenith_angle: Option<Angle>,
) -> f64 {
    let t_star = star_temperature.value();
    if t_star <= 0.0 || !t_star.is_finite() {
        return 1.0;
    }

    let l_min = UV_WAVELENGTH_MIN_M;
    let l_max = UV_WAVELENGTH_MAX_M;
    let n = 40;
    let dl = (l_max - l_min) / (n as f64);

    let mut num = 0.0;
    let mut den = 0.0;

    for i in 0..=n {
        let l = l_min + (i as f64) * dl;
        let wl = Wavelength::new(l);
        let b_lambda = planck_spectral_radiance(wl, star_temperature);
        let w_dna = dna_damage_spectral_weight(wl);
        let weight = if i == 0 || i == n { 0.5 } else { 1.0 };

        let rayleigh_coeff = rayleigh_scattering_coefficient(
            wl,
            gas_optical_properties.refractivity_stp(),
            gas_optical_properties.king_factor(),
            pressure,
            temperature,
        );
        let abs_coeff = absorption_coefficient(gas_optical_properties, wl, pressure, temperature);
        let tau_v = vertical_optical_depth(
            rayleigh_coeff,
            mie_coeff,
            abs_coeff,
            scale_height,
            aerosol_scale_height,
        );
        let tau = match zenith_angle {
            Some(z) => slant_optical_depth(tau_v, z),
            None => tau_v,
        };
        let transmittance = atmospheric_transmittance(tau);

        num += weight * b_lambda * w_dna * transmittance * dl;
        den += weight * b_lambda * w_dna * dl;
    }

    if den <= 0.0 || !den.is_finite() {
        1.0
    } else {
        (num / den).clamp(0.0, 1.0)
    }
}

pub fn uv_band_transmittance(
    star_temperature: Temperature,
    min_wavelength: Wavelength,
    max_wavelength: Wavelength,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    scale_height: Length,
    aerosol_scale_height: Length,
    mie_coeff: f64,
    zenith_angle: Option<Angle>,
) -> f64 {
    let t_star = star_temperature.value();
    let l_min = min_wavelength.value();
    let l_max = max_wavelength.value();

    if t_star <= 0.0 || l_min <= 0.0 || l_max <= l_min || !t_star.is_finite() {
        return 1.0;
    }

    let n = 20;
    let dl = (l_max - l_min) / (n as f64);
    let mut num = 0.0;
    let mut den = 0.0;

    for i in 0..=n {
        let l = l_min + (i as f64) * dl;
        let wl = Wavelength::new(l);
        let b_lambda = planck_spectral_radiance(wl, star_temperature);
        let weight = if i == 0 || i == n { 0.5 } else { 1.0 };

        let rayleigh_coeff = rayleigh_scattering_coefficient(
            wl,
            gas_optical_properties.refractivity_stp(),
            gas_optical_properties.king_factor(),
            pressure,
            temperature,
        );
        let abs_coeff = absorption_coefficient(gas_optical_properties, wl, pressure, temperature);
        let tau_v = vertical_optical_depth(
            rayleigh_coeff,
            mie_coeff,
            abs_coeff,
            scale_height,
            aerosol_scale_height,
        );
        let tau = match zenith_angle {
            Some(z) => slant_optical_depth(tau_v, z),
            None => tau_v,
        };
        let transmittance = atmospheric_transmittance(tau);

        num += weight * b_lambda * transmittance * dl;
        den += weight * b_lambda * dl;
    }

    if den <= 0.0 || !den.is_finite() {
        1.0
    } else {
        (num / den).clamp(0.0, 1.0)
    }
}

pub fn unshielded_dna_effective_uv_fraction(star_temperature: Temperature) -> f64 {
    let t = star_temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return 0.0;
    }

    let total_flux = STEFAN_BOLTZMANN_CONSTANT * t.powi(4);
    if total_flux <= 0.0 {
        return 0.0;
    }

    let l_min = UV_WAVELENGTH_MIN_M;
    let l_max = UV_WAVELENGTH_MAX_M;
    let n = 40;
    let dl = (l_max - l_min) / (n as f64);
    let mut sum = 0.0;

    for i in 0..=n {
        let l = l_min + (i as f64) * dl;
        let wl = Wavelength::new(l);
        let b_lambda = planck_spectral_radiance(wl, star_temperature);
        let w_dna = dna_damage_spectral_weight(wl);
        let weight = if i == 0 || i == n { 0.5 } else { 1.0 };
        sum += weight * b_lambda * w_dna * dl;
    }

    (PI * sum / total_flux).max(0.0)
}

pub fn unshielded_dna_effective_uv_irradiance(
    total_toa_irradiance: Irradiance,
    star_temperature: Temperature,
) -> Irradiance {
    let frac = unshielded_dna_effective_uv_fraction(star_temperature);
    Irradiance::new(total_toa_irradiance.value() * frac)
}

pub fn surface_dna_effective_uv_irradiance(
    total_toa_irradiance: Irradiance,
    star_temperature: Temperature,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    scale_height: Length,
    aerosol_scale_height: Length,
    mie_coeff: f64,
    zenith_angle: Option<Angle>,
) -> Irradiance {
    let toa_eff = unshielded_dna_effective_uv_irradiance(total_toa_irradiance, star_temperature);
    let trans = dna_weighted_uv_transmittance(
        star_temperature,
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );
    Irradiance::new(toa_eff.value() * trans)
}

pub fn dna_uv_shielding_efficiency(
    star_temperature: Temperature,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    scale_height: Length,
    aerosol_scale_height: Length,
    mie_coeff: f64,
    zenith_angle: Option<Angle>,
) -> f64 {
    let trans = dna_weighted_uv_transmittance(
        star_temperature,
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );
    (1.0 - trans).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UvPhotobiologyResult {
    pub toa_effective_uv_irradiance: Irradiance,
    pub surface_effective_uv_irradiance: Irradiance,
    pub dna_shielding_efficiency: f64,
    pub uvc_transmittance: f64,
    pub uvb_transmittance: f64,
    pub uva_transmittance: f64,
}

pub fn evaluate_uv_photobiology(
    total_toa_irradiance: Irradiance,
    star_temperature: Temperature,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    scale_height: Length,
    aerosol_scale_height: Length,
    mie_coeff: f64,
    zenith_angle: Option<Angle>,
) -> UvPhotobiologyResult {
    let toa_eff = unshielded_dna_effective_uv_irradiance(total_toa_irradiance, star_temperature);
    let trans_dna = dna_weighted_uv_transmittance(
        star_temperature,
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );
    let surf_eff = Irradiance::new(toa_eff.value() * trans_dna);
    let shielding = (1.0 - trans_dna).clamp(0.0, 1.0);

    let uvc = uv_band_transmittance(
        star_temperature,
        Wavelength::new(UV_WAVELENGTH_MIN_M),
        Wavelength::new(UVC_WAVELENGTH_MAX_M),
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );

    let uvb = uv_band_transmittance(
        star_temperature,
        Wavelength::new(UVC_WAVELENGTH_MAX_M),
        Wavelength::new(UVB_WAVELENGTH_MAX_M),
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );

    let uva = uv_band_transmittance(
        star_temperature,
        Wavelength::new(UVB_WAVELENGTH_MAX_M),
        Wavelength::new(UV_WAVELENGTH_MAX_M),
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );

    UvPhotobiologyResult {
        toa_effective_uv_irradiance: toa_eff,
        surface_effective_uv_irradiance: surf_eff,
        dna_shielding_efficiency: shielding,
        uvc_transmittance: uvc,
        uvb_transmittance: uvb,
        uva_transmittance: uva,
    }
}