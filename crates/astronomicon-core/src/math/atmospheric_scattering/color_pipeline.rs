use crate::math::atmospheric_scattering::profiles::{
    AtmosphericRaymarchConfig, SphericalAtmosphere,
};
use crate::math::atmospheric_scattering::single_scattering::{
    single_scattering_spectral_radiance, stellar_disk_integrated_single_scattering,
};
use crate::math::colorimetry::{
    ColorXYZ, cie_color_matching_functions, exposure_tone_map, linear_to_srgb_gamma,
    xyz_to_linear_srgb,
};
use crate::math::radiation::planck_spectral_radiance;
use crate::units::constants::{CIE_WAVELENGTH_MAX_M, CIE_WAVELENGTH_MIN_M, CIE_WAVELENGTH_STEP_M};
use crate::units::{Angle, ColorRGB, Temperature, Vector3, Wavelength};
use std::f64::consts::PI;

pub fn integrate_spectrum_to_xyz<F>(
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    mut evaluate_radiance: F,
) -> ColorXYZ
where
    F: FnMut(Wavelength, f64) -> f64,
{
    let t_sun = solar_temperature.value();
    let theta_sun = solar_angular_radius_rad.clamp(0.0, PI / 2.0);

    if t_sun <= 0.0 || theta_sun <= 0.0 || !t_sun.is_finite() || !theta_sun.is_finite() {
        return ColorXYZ::zero();
    }

    let solid_angle_sun = PI * theta_sun.sin().powi(2);
    let step = CIE_WAVELENGTH_STEP_M;
    let mut accumulated = ColorXYZ::zero();
    let mut lambda_m = CIE_WAVELENGTH_MIN_M;

    while lambda_m <= CIE_WAVELENGTH_MAX_M {
        let wavelength = Wavelength::new(lambda_m);
        let b_lambda = planck_spectral_radiance(wavelength, solar_temperature);
        let solar_irradiance = b_lambda * solid_angle_sun;

        if solar_irradiance > 0.0 && solar_irradiance.is_finite() {
            let radiance = evaluate_radiance(wavelength, solar_irradiance);
            let cmf = cie_color_matching_functions(wavelength);
            accumulated = accumulated + cmf * radiance;
        }

        lambda_m += step;
    }

    accumulated * step
}

pub fn color_xyz_to_exposed_srgb(xyz: ColorXYZ, exposure: f64) -> ColorRGB {
    let linear_rgb = xyz_to_linear_srgb(xyz);
    let exposed = exposure_tone_map(linear_rgb, exposure);
    linear_to_srgb_gamma(exposed)
}

pub fn sample_rgb_channels<T, F>(solar_irradiance_rgb: ColorRGB, mut evaluate: F) -> (T, T, T)
where
    F: FnMut(Wavelength, f64) -> T,
{
    let w_r = Wavelength::new(680.0e-9);
    let w_g = Wavelength::new(550.0e-9);
    let w_b = Wavelength::new(440.0e-9);
    let res_r = evaluate(w_r, solar_irradiance_rgb.r());
    let res_g = evaluate(w_g, solar_irradiance_rgb.g());
    let res_b = evaluate(w_b, solar_irradiance_rgb.b());
    (res_r, res_g, res_b)
}

pub fn sample_2channel_rgb_fast<F>(
    solar_irradiance_rgb: ColorRGB,
    mut evaluate: F,
) -> (ColorRGB, ColorRGB)
where
    F: FnMut(Wavelength, f64) -> (f64, f64),
{
    let (r, g, b) = sample_rgb_channels(solar_irradiance_rgb, |w, irr| evaluate(w, irr));
    let channel_0 = ColorRGB::new(r.0, g.0, b.0);
    let channel_1 = ColorRGB::new(r.1, g.1, b.1);
    (channel_0, channel_1)
}

pub fn sample_3channel_rgb_fast<F>(
    solar_irradiance_rgb: ColorRGB,
    mut evaluate: F,
) -> (ColorRGB, ColorRGB, ColorRGB)
where
    F: FnMut(Wavelength, f64) -> (f64, f64, f64),
{
    let (r, g, b) = sample_rgb_channels(solar_irradiance_rgb, |w, irr| evaluate(w, irr));
    let channel_0 = ColorRGB::new(r.0, g.0, b.0);
    let channel_1 = ColorRGB::new(r.1, g.1, b.1);
    let channel_2 = ColorRGB::new(r.2, g.2, b.2);
    (channel_0, channel_1, channel_2)
}

pub fn spherical_sky_color_xyz(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> ColorXYZ {
    integrate_spectrum_to_xyz(
        solar_temperature,
        solar_angular_radius_rad,
        |wavelength, solar_irradiance| {
            single_scattering_spectral_radiance(
                ray_origin,
                ray_dir,
                sun_dir,
                solar_irradiance,
                wavelength,
                atmosphere,
                config,
            )
            .in_scattered_radiance
        },
    )
}

pub fn spherical_sky_color_rgb(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    exposure: f64,
    config: &AtmosphericRaymarchConfig,
) -> ColorRGB {
    let xyz = spherical_sky_color_xyz(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_temperature,
        solar_angular_radius_rad,
        atmosphere,
        config,
    );
    color_xyz_to_exposed_srgb(xyz, exposure)
}

pub fn spherical_sky_rgb_fast(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_irradiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> (ColorRGB, ColorRGB) {
    sample_2channel_rgb_fast(solar_irradiance_rgb, |w, irr| {
        let res = single_scattering_spectral_radiance(
            ray_origin, ray_dir, sun_dir, irr, w, atmosphere, config,
        );
        (res.in_scattered_radiance, res.transmittance)
    })
}

pub fn stellar_disk_sky_color_xyz(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_temperature: Temperature,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> ColorXYZ {
    integrate_spectrum_to_xyz(
        solar_temperature,
        star_angular_radius.value(),
        |wavelength, solar_irradiance| {
            stellar_disk_integrated_single_scattering(
                ray_origin,
                ray_dir,
                geometric_sun_dir,
                star_angular_radius,
                solar_irradiance,
                wavelength,
                atmosphere,
                config,
                disk_samples,
                limb_darkening_coeff,
            )
            .in_scattered_radiance
        },
    )
}

pub fn stellar_disk_sky_color_rgb(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_temperature: Temperature,
    atmosphere: &SphericalAtmosphere,
    exposure: f64,
    config: &AtmosphericRaymarchConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> ColorRGB {
    let xyz = stellar_disk_sky_color_xyz(
        ray_origin,
        ray_dir,
        geometric_sun_dir,
        star_angular_radius,
        solar_temperature,
        atmosphere,
        config,
        disk_samples,
        limb_darkening_coeff,
    );
    color_xyz_to_exposed_srgb(xyz, exposure)
}

pub fn stellar_disk_rgb_fast(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_irradiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> (ColorRGB, ColorRGB) {
    sample_2channel_rgb_fast(solar_irradiance_rgb, |w, irr| {
        let res = stellar_disk_integrated_single_scattering(
            ray_origin,
            ray_dir,
            geometric_sun_dir,
            star_angular_radius,
            irr,
            w,
            atmosphere,
            config,
            disk_samples,
            limb_darkening_coeff,
        );
        (res.in_scattered_radiance, res.transmittance)
    })
}
