use crate::sky::SkyColorDiagnostic;
use astronomicon_core::math::colorimetry::{
    chromatically_adapt_xyz, linear_to_srgb_gamma, reinhard_extended_tone_map, xyz_to_linear_srgb,
    ColorXYZ,
};

pub fn process_sky_colors(
    xyz_zenith: ColorXYZ,
    xyz_horizon: ColorXYZ,
    xyz_sunset: ColorXYZ,
    xyz_sun_toa: ColorXYZ,
) -> SkyColorDiagnostic {
    let d65_white = ColorXYZ::new(0.95047, 1.0, 1.08883);
    let star_white = if xyz_sun_toa.y() > 1e-12 {
        xyz_sun_toa / xyz_sun_toa.y()
    } else {
        d65_white
    };

    let adapt = |xyz: ColorXYZ| chromatically_adapt_xyz(xyz, star_white, d65_white);

    let xyz_zenith_adapted = adapt(xyz_zenith);
    let xyz_horizon_adapted = adapt(xyz_horizon);
    let xyz_sunset_adapted = adapt(xyz_sunset);

    let rgb_zenith_linear = xyz_to_linear_srgb(xyz_zenith_adapted);
    let max_zenith_ch = rgb_zenith_linear
        .r()
        .max(rgb_zenith_linear.g())
        .max(rgb_zenith_linear.b());

    let target_peak_channel = 0.78;
    let exposure = if max_zenith_ch > 1e-12 {
        target_peak_channel / max_zenith_ch
    } else if xyz_zenith_adapted.y() > 1e-12 {
        0.35 / xyz_zenith_adapted.y()
    } else {
        1.0
    };

    let process_color = |xyz: ColorXYZ| {
        let rgb = xyz_to_linear_srgb(xyz);
        let exposed = reinhard_extended_tone_map(rgb * exposure, 4.0);
        linear_to_srgb_gamma(exposed)
    };

    SkyColorDiagnostic {
        zenith_color: process_color(xyz_zenith_adapted),
        horizon_color: process_color(xyz_horizon_adapted),
        sunset_color: process_color(xyz_sunset_adapted),
    }
}
