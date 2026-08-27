use crate::chemistry::optics::GasOpticalProperties;
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

pub const PAR_WAVELENGTH_MIN_M: f64 = 400.0e-9;
pub const PAR_WAVELENGTH_MAX_M: f64 = 700.0e-9;
pub const MAX_THEORETICAL_PHOTOSYNTHETIC_EFFICIENCY: f64 = 0.11;

pub fn blackbody_spectral_band_flux(
    star_temperature: Temperature,
    min_wavelength: Wavelength,
    max_wavelength: Wavelength,
    step_count: usize,
) -> f64 {
    let t = star_temperature.value();
    let l_min = min_wavelength.value();
    let l_max = max_wavelength.value();

    if t <= 0.0
        || l_min <= 0.0
        || l_max <= l_min
        || !t.is_finite()
        || !l_min.is_finite()
        || !l_max.is_finite()
    {
        return 0.0;
    }

    let n = step_count.max(10);
    let dl = (l_max - l_min) / (n as f64);
    let mut sum = 0.0;

    for i in 0..=n {
        let l = l_min + (i as f64) * dl;
        let radiance = planck_spectral_radiance(Wavelength::new(l), star_temperature);
        let weight = if i == 0 || i == n { 0.5 } else { 1.0 };
        sum += weight * radiance * dl;
    }

    PI * sum
}

pub fn par_spectral_fraction(star_temperature: Temperature) -> f64 {
    let t = star_temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return 0.0;
    }

    let total_flux = STEFAN_BOLTZMANN_CONSTANT * t.powi(4);
    if total_flux <= 0.0 || !total_flux.is_finite() {
        return 0.0;
    }

    let par_flux = blackbody_spectral_band_flux(
        star_temperature,
        Wavelength::new(PAR_WAVELENGTH_MIN_M),
        Wavelength::new(PAR_WAVELENGTH_MAX_M),
        40,
    );

    (par_flux / total_flux).clamp(0.0, 1.0)
}

pub fn top_of_atmosphere_par_irradiance(
    total_toa_irradiance: Irradiance,
    star_temperature: Temperature,
) -> Irradiance {
    let frac = par_spectral_fraction(star_temperature);
    Irradiance::new(total_toa_irradiance.value() * frac)
}

pub fn par_spectral_transmittance_weighted(
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

    let l_min = PAR_WAVELENGTH_MIN_M;
    let l_max = PAR_WAVELENGTH_MAX_M;
    let n = 30;
    let dl = (l_max - l_min) / (n as f64);

    let mut num_sum = 0.0;
    let mut den_sum = 0.0;

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

        num_sum += weight * b_lambda * transmittance * dl;
        den_sum += weight * b_lambda * dl;
    }

    if den_sum <= 0.0 || !den_sum.is_finite() {
        1.0
    } else {
        (num_sum / den_sum).clamp(0.0, 1.0)
    }
}

pub fn surface_par_irradiance(
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
    let toa_par = top_of_atmosphere_par_irradiance(total_toa_irradiance, star_temperature);
    let transmittance = par_spectral_transmittance_weighted(
        star_temperature,
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );
    Irradiance::new(toa_par.value() * transmittance)
}

pub fn theoretical_max_biomass_energy_flux(surface_par: Irradiance) -> Irradiance {
    Irradiance::new(surface_par.value() * MAX_THEORETICAL_PHOTOSYNTHETIC_EFFICIENCY)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhotosyntheticFluxSummary {
    pub toa_par_irradiance: Irradiance,
    pub surface_par_irradiance: Irradiance,
    pub par_fraction_of_total: f64,
    pub atmospheric_par_transmittance: f64,
    pub max_biomass_energy_flux: Irradiance,
}

pub fn evaluate_photosynthetic_flux(
    total_toa_irradiance: Irradiance,
    star_temperature: Temperature,
    gas_optical_properties: &GasOpticalProperties,
    pressure: Pressure,
    temperature: Temperature,
    scale_height: Length,
    aerosol_scale_height: Length,
    mie_coeff: f64,
    zenith_angle: Option<Angle>,
) -> PhotosyntheticFluxSummary {
    let par_frac = par_spectral_fraction(star_temperature);
    let toa_par = Irradiance::new(total_toa_irradiance.value() * par_frac);
    let trans = par_spectral_transmittance_weighted(
        star_temperature,
        gas_optical_properties,
        pressure,
        temperature,
        scale_height,
        aerosol_scale_height,
        mie_coeff,
        zenith_angle,
    );
    let surf_par = Irradiance::new(toa_par.value() * trans);
    let max_biomass = theoretical_max_biomass_energy_flux(surf_par);

    PhotosyntheticFluxSummary {
        toa_par_irradiance: toa_par,
        surface_par_irradiance: surf_par,
        par_fraction_of_total: par_frac,
        atmospheric_par_transmittance: trans,
        max_biomass_energy_flux: max_biomass,
    }
}