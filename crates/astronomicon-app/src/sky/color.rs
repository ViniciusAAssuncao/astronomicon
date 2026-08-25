use crate::sky::exposure::expose_and_tone_map_radiance;
use crate::sky::radiance::{SkyRadianceDiagnostic, SpectralRadiance};
use astronomicon_core::units::ColorRGB;
use serde::{Deserialize, Serialize};

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

pub fn tone_mapped_to_color_rgb(r_mapped: f64, g_mapped: f64, b_mapped: f64) -> ColorRGB {
    ColorRGB::new(
        linear_to_srgb(r_mapped),
        linear_to_srgb(g_mapped),
        linear_to_srgb(b_mapped),
    )
}

pub fn spectral_radiance_to_color_rgb(radiance: SpectralRadiance) -> ColorRGB {
    let (r, g, b) = expose_and_tone_map_radiance(radiance);
    tone_mapped_to_color_rgb(r, g, b)
}

pub fn process_sky_colors_from_radiances(
    radiances: &SkyRadianceDiagnostic,
) -> SkyColorDiagnostic {
    let (zr, zg, zb) = expose_and_tone_map_radiance(radiances.zenith_radiance);
    let (hr, hg, hb) = expose_and_tone_map_radiance(radiances.horizon_radiance);
    let (sr, sg, sb) = expose_and_tone_map_radiance(radiances.sunset_radiance);

    SkyColorDiagnostic {
        zenith_color: tone_mapped_to_color_rgb(zr, zg, zb),
        horizon_color: tone_mapped_to_color_rgb(hr, hg, hb),
        sunset_color: tone_mapped_to_color_rgb(sr, sg, sb),
    }
}
