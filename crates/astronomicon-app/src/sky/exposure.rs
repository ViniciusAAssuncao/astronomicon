use crate::sky::radiance::SpectralRadiance;
use astronomicon_core::units::constants::MAXIMUM_LUMINOUS_EFFICACY;
use astronomicon_core::units::{Illuminance, Luminance};
use std::f64::consts::PI;

pub const EARTH_STANDARD_TSI_W_M2: f64 = 1360.8;
pub const CAMERA_EXPOSURE_CALIBRATION_K: f64 = PI / EARTH_STANDARD_TSI_W_M2;
pub const ISO_2720_INCIDENT_CALIBRATION_C: f64 = 250.0;
pub const ISO_2720_REFLECTED_CALIBRATION_K: f64 = 12.5;
pub const SUNNY_16_APERTURE_NUMBER: f64 = 16.0;
pub const SUNNY_16_ISO_SPEED: f64 = 100.0;
pub const DEFAULT_AUTO_EXPOSURE_KEY: f64 = 0.36;

pub fn sunny_16_exposure_value() -> f64 {
    let n = SUNNY_16_APERTURE_NUMBER;
    let t = 1.0 / SUNNY_16_ISO_SPEED;
    (n * n / t).log2()
}

pub fn ev100_from_illuminance(illuminance: Illuminance) -> f64 {
    let lux = illuminance.value();
    if lux <= 0.0 || !lux.is_finite() {
        0.0
    } else {
        (lux * SUNNY_16_ISO_SPEED / ISO_2720_INCIDENT_CALIBRATION_C).log2()
    }
}

pub fn ev100_from_luminance(luminance: Luminance) -> f64 {
    let cd_m2 = luminance.value();
    if cd_m2 <= 0.0 || !cd_m2.is_finite() {
        0.0
    } else {
        (cd_m2 * SUNNY_16_ISO_SPEED / ISO_2720_REFLECTED_CALIBRATION_K).log2()
    }
}

pub fn exposure_factor_from_ev(ev: f64) -> f64 {
    if !ev.is_finite() {
        0.0
    } else {
        2.0_f64.powf(-ev)
    }
}

pub fn expose_spectral_radiance(radiance: SpectralRadiance) -> (f64, f64, f64) {
    (
        radiance.r * CAMERA_EXPOSURE_CALIBRATION_K,
        radiance.g * CAMERA_EXPOSURE_CALIBRATION_K,
        radiance.b * CAMERA_EXPOSURE_CALIBRATION_K,
    )
}

pub fn auto_exposure_factor(adaptation_luminance: Luminance, key_value: f64) -> f64 {
    let l = adaptation_luminance.value();
    if l <= 0.0 || !l.is_finite() {
        CAMERA_EXPOSURE_CALIBRATION_K
    } else {
        let key = if key_value.is_finite() && key_value > 0.0 {
            key_value
        } else {
            DEFAULT_AUTO_EXPOSURE_KEY
        };
        (key * MAXIMUM_LUMINOUS_EFFICACY) / l
    }
}

pub fn auto_expose_spectral_radiance(
    radiance: SpectralRadiance,
    adaptation_luminance: Luminance,
    key_value: f64,
) -> (f64, f64, f64) {
    let factor = auto_exposure_factor(adaptation_luminance, key_value);
    (radiance.r * factor, radiance.g * factor, radiance.b * factor)
}

pub fn reinhard_tone_map(linear: f64) -> f64 {
    if linear <= 0.0 || !linear.is_finite() {
        0.0
    } else {
        linear / (1.0 + linear)
    }
}

pub fn reinhard_extended_tone_map(linear: f64, max_white: f64) -> f64 {
    if linear <= 0.0 || !linear.is_finite() {
        0.0
    } else if max_white <= 0.0 || !max_white.is_finite() {
        linear / (1.0 + linear)
    } else {
        let w2 = max_white * max_white;
        let num = linear * (1.0 + linear / w2);
        let den = 1.0 + linear;
        (num / den).clamp(0.0, 1.0)
    }
}

pub fn reinhard_tone_map_rgb(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    (
        reinhard_tone_map(r),
        reinhard_tone_map(g),
        reinhard_tone_map(b),
    )
}

pub fn expose_and_tone_map_radiance(radiance: SpectralRadiance) -> (f64, f64, f64) {
    let (r_exp, g_exp, b_exp) = expose_spectral_radiance(radiance);
    reinhard_tone_map_rgb(r_exp, g_exp, b_exp)
}

pub fn auto_expose_and_tone_map_radiance(
    radiance: SpectralRadiance,
    adaptation_luminance: Luminance,
    key_value: f64,
) -> (f64, f64, f64) {
    let (r_exp, g_exp, b_exp) =
        auto_expose_spectral_radiance(radiance, adaptation_luminance, key_value);
    reinhard_tone_map_rgb(r_exp, g_exp, b_exp)
}

pub fn human_eye_adapt_and_tone_map_radiance(
    radiance: SpectralRadiance,
    adaptation_luminance: Luminance,
) -> (f64, f64, f64) {
    auto_expose_and_tone_map_radiance(radiance, adaptation_luminance, DEFAULT_AUTO_EXPOSURE_KEY)
}