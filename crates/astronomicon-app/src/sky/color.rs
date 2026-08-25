use crate::sky::radiance::{SkyRadianceDiagnostic, SpectralRadiance};
use astronomicon_core::units::ColorRGB;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyColorDiagnostic {
    pub zenith_color: ColorRGB,
    pub horizon_color: ColorRGB,
    pub sunset_color: ColorRGB,
}

pub fn linear_to_srgb(linear: f64) -> f64 {
    let x = linear.clamp(0.0, 1.0);
    if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

pub fn spectral_radiance_to_color_rgb(
    radiance: SpectralRadiance,
    solar_irradiance: SpectralRadiance,
) -> ColorRGB {
    let r_lin = (PI * radiance.r / solar_irradiance.r.max(1e-12)).clamp(0.0, 1.0);
    let g_lin = (PI * radiance.g / solar_irradiance.g.max(1e-12)).clamp(0.0, 1.0);
    let b_lin = (PI * radiance.b / solar_irradiance.b.max(1e-12)).clamp(0.0, 1.0);

    ColorRGB::new(
        linear_to_srgb(r_lin),
        linear_to_srgb(g_lin),
        linear_to_srgb(b_lin),
    )
}

pub fn process_sky_colors_from_radiances(
    radiances: &SkyRadianceDiagnostic,
) -> SkyColorDiagnostic {
    let zenith = spectral_radiance_to_color_rgb(
        radiances.zenith_radiance,
        radiances.solar_irradiance,
    );
    let horizon = spectral_radiance_to_color_rgb(
        radiances.horizon_radiance,
        radiances.solar_irradiance,
    );
    let sunset = spectral_radiance_to_color_rgb(
        radiances.sunset_radiance,
        radiances.solar_irradiance,
    );

    SkyColorDiagnostic {
        zenith_color: zenith,
        horizon_color: horizon,
        sunset_color: sunset,
    }
}