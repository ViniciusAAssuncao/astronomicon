use crate::math::atmospheric_scattering::{
    SphericalAtmosphere, color_xyz_to_exposed_srgb, integrate_spectrum_to_xyz,
    sample_3channel_rgb_fast,
};
use crate::math::colorimetry::ColorXYZ;
use crate::math::scattering::config::MultipleScatteringConfig;
use crate::math::scattering::radiance::{
    multiple_scattering_spectral_radiance, multiple_scattering_stellar_disk_spectral_radiance,
};
use crate::units::{Angle, ColorRGB, Temperature, Vector3};

pub fn multiple_scattering_sky_color_xyz(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig,
) -> ColorXYZ {
    integrate_spectrum_to_xyz(
        solar_temperature,
        solar_angular_radius_rad,
        |wavelength, solar_irradiance| {
            multiple_scattering_spectral_radiance(
                ray_origin,
                ray_dir,
                sun_dir,
                solar_irradiance,
                wavelength,
                atmosphere,
                config,
            )
            .total_radiance
        },
    )
}

pub fn multiple_scattering_sky_color_rgb(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_temperature: Temperature,
    solar_angular_radius_rad: f64,
    atmosphere: &SphericalAtmosphere,
    exposure: f64,
    config: &MultipleScatteringConfig,
) -> ColorRGB {
    let xyz = multiple_scattering_sky_color_xyz(
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

pub fn multiple_scattering_sky_rgb_fast(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_irradiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig,
) -> (ColorRGB, ColorRGB, ColorRGB) {
    sample_3channel_rgb_fast(solar_irradiance_rgb, |w, irr| {
        let res = multiple_scattering_spectral_radiance(
            ray_origin, ray_dir, sun_dir, irr, w, atmosphere, config,
        );
        (
            res.single_scattered_radiance,
            res.multiple_scattered_radiance,
            res.transmittance,
        )
    })
}

pub fn multiple_scattering_stellar_disk_sky_color_xyz(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_temperature: Temperature,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> ColorXYZ {
    integrate_spectrum_to_xyz(
        solar_temperature,
        star_angular_radius.value(),
        |wavelength, solar_irradiance| {
            multiple_scattering_stellar_disk_spectral_radiance(
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
            .total_radiance
        },
    )
}

pub fn multiple_scattering_stellar_disk_sky_color_rgb(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_temperature: Temperature,
    atmosphere: &SphericalAtmosphere,
    exposure: f64,
    config: &MultipleScatteringConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> ColorRGB {
    let xyz = multiple_scattering_stellar_disk_sky_color_xyz(
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

pub fn multiple_scattering_stellar_disk_rgb_fast(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_irradiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &MultipleScatteringConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> (ColorRGB, ColorRGB, ColorRGB) {
    sample_3channel_rgb_fast(solar_irradiance_rgb, |w, irr| {
        let res = multiple_scattering_stellar_disk_spectral_radiance(
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
        (
            res.single_scattered_radiance,
            res.multiple_scattered_radiance,
            res.transmittance,
        )
    })
}
