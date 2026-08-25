use crate::sky::exposure::{
    expose_and_tone_map_radiance, human_eye_adapt_and_tone_map_radiance,
};
use crate::sky::radiance::{SkyRadianceDiagnostic, SpectralRadiance};
use astronomicon_core::math::radiometry::photopic_luminance;
use astronomicon_core::units::{ColorRGB, Luminance};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyColorDiagnostic {
    pub zenith_color: ColorRGB,
    pub horizon_color: ColorRGB,
    pub sunset_color: ColorRGB,
    pub sunset_halo_color: ColorRGB,
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

pub fn spectral_radiance_to_adapted_color_rgb(
    radiance: SpectralRadiance,
    adaptation_luminance: Luminance,
) -> ColorRGB {
    let (r, g, b) = human_eye_adapt_and_tone_map_radiance(radiance, adaptation_luminance);
    tone_mapped_to_color_rgb(r, g, b)
}

pub fn process_sky_colors_from_radiances(
    radiances: &SkyRadianceDiagnostic,
) -> SkyColorDiagnostic {
    let (zr, zg, zb) = expose_and_tone_map_radiance(radiances.zenith_radiance);
    let (hr, hg, hb) = expose_and_tone_map_radiance(radiances.horizon_radiance);
    let (sr, sg, sb) = expose_and_tone_map_radiance(radiances.sunset_radiance);
    let (shr, shg, shb) = expose_and_tone_map_radiance(radiances.sunset_halo_radiance);

    SkyColorDiagnostic {
        zenith_color: tone_mapped_to_color_rgb(zr, zg, zb),
        horizon_color: tone_mapped_to_color_rgb(hr, hg, hb),
        sunset_color: tone_mapped_to_color_rgb(sr, sg, sb),
        sunset_halo_color: tone_mapped_to_color_rgb(shr, shg, shb),
    }
}

pub fn process_human_eye_sky_colors(
    radiances: &SkyRadianceDiagnostic,
    adaptation_luminance: Luminance,
) -> SkyColorDiagnostic {
    let (zr, zg, zb) =
        human_eye_adapt_and_tone_map_radiance(radiances.zenith_radiance, adaptation_luminance);
    let (hr, hg, hb) =
        human_eye_adapt_and_tone_map_radiance(radiances.horizon_radiance, adaptation_luminance);
    let (sr, sg, sb) =
        human_eye_adapt_and_tone_map_radiance(radiances.sunset_radiance, adaptation_luminance);
    let (shr, shg, shb) = human_eye_adapt_and_tone_map_radiance(
        radiances.sunset_halo_radiance,
        adaptation_luminance,
    );

    SkyColorDiagnostic {
        zenith_color: tone_mapped_to_color_rgb(zr, zg, zb),
        horizon_color: tone_mapped_to_color_rgb(hr, hg, hb),
        sunset_color: tone_mapped_to_color_rgb(sr, sg, sb),
        sunset_halo_color: tone_mapped_to_color_rgb(shr, shg, shb),
    }
}

pub fn process_locally_adapted_sky_colors(
    radiances: &SkyRadianceDiagnostic,
) -> SkyColorDiagnostic {
    let z_lum = photopic_luminance(
        radiances.zenith_radiance.r,
        radiances.zenith_radiance.g,
        radiances.zenith_radiance.b,
    );
    let h_lum = photopic_luminance(
        radiances.horizon_radiance.r,
        radiances.horizon_radiance.g,
        radiances.horizon_radiance.b,
    );
    let s_lum = photopic_luminance(
        radiances.sunset_radiance.r,
        radiances.sunset_radiance.g,
        radiances.sunset_radiance.b,
    );
    let sh_lum = photopic_luminance(
        radiances.sunset_halo_radiance.r,
        radiances.sunset_halo_radiance.g,
        radiances.sunset_halo_radiance.b,
    );

    let (zr, zg, zb) = human_eye_adapt_and_tone_map_radiance(radiances.zenith_radiance, z_lum);
    let (hr, hg, hb) = human_eye_adapt_and_tone_map_radiance(radiances.horizon_radiance, h_lum);
    let (sr, sg, sb) = human_eye_adapt_and_tone_map_radiance(radiances.sunset_radiance, s_lum);
    let (shr, shg, shb) =
        human_eye_adapt_and_tone_map_radiance(radiances.sunset_halo_radiance, sh_lum);

    SkyColorDiagnostic {
        zenith_color: tone_mapped_to_color_rgb(zr, zg, zb),
        horizon_color: tone_mapped_to_color_rgb(hr, hg, hb),
        sunset_color: tone_mapped_to_color_rgb(sr, sg, sb),
        sunset_halo_color: tone_mapped_to_color_rgb(shr, shg, shb),
    }
}