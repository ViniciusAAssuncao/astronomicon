use astronomicon_core::math::aerosol::refractivity_at_temperature_pressure;
use astronomicon_core::math::atmospheric_scattering::SphericalAtmosphere;
use astronomicon_core::math::colorimetry::{ColorXYZ, cie_color_matching_functions};
use astronomicon_core::math::optics::refracted_sun_direction;
use astronomicon_core::math::radiation::planck_spectral_radiance;
use astronomicon_core::math::scattering::{
    MultipleScatteringConfig, multiple_scattering_spectral_radiance,
    multiple_scattering_stellar_disk_spectral_radiance,
};
use astronomicon_core::units::constants::{
    CIE_WAVELENGTH_MAX_M, CIE_WAVELENGTH_MIN_M, CIE_WAVELENGTH_STEP_M,
};
use astronomicon_core::units::{Angle, Length, Pressure, Temperature, Vector3, Wavelength};
use std::f64::consts::PI;

pub struct IntegratedSpectralRadiances {
    pub xyz_zenith: ColorXYZ,
    pub xyz_horizon: ColorXYZ,
    pub xyz_sunset: ColorXYZ,
    pub xyz_sun_toa: ColorXYZ,
}

pub fn cmf_eval(wavelength: Wavelength) -> ColorXYZ {
    cie_color_matching_functions(wavelength)
}

pub fn integrate_sky_spectrum(
    atmosphere: &SphericalAtmosphere,
    ms_config: &MultipleScatteringConfig,
    star_temp: Temperature,
    solid_angle_sun: f64,
    star_angular_radius: Angle,
    eq_radius: Length,
    scale_h: Length,
    refr_stp: f64,
    surf_temp: Temperature,
    surf_press: Pressure,
) -> IntegratedSpectralRadiances {
    let ray_origin = Vector3::new(0.0, eq_radius.value(), 0.0);
    let up = ray_origin.normalized();
    let refr_actual = refractivity_at_temperature_pressure(refr_stp, surf_temp, surf_press);

    let view_zenith = Vector3::new(0.0, 1.0, 0.0);
    let sun_dir_day = Vector3::new((PI / 4.0).sin(), (PI / 4.0).cos(), 0.0).normalized();
    let s_refracted_day = refracted_sun_direction(sun_dir_day, up, refr_actual, scale_h, eq_radius);

    let view_horizon = Vector3::new(1.0, 0.0, 0.0);
    let view_sunset = Vector3::new(1.0, 0.0, 0.0);
    let sun_dir_sunset = Vector3::new(1.0, 0.0, 0.0);

    let step = CIE_WAVELENGTH_STEP_M;
    let mut xyz_zenith = ColorXYZ::zero();
    let mut xyz_horizon = ColorXYZ::zero();
    let mut xyz_sunset = ColorXYZ::zero();
    let mut xyz_sun_toa = ColorXYZ::zero();

    let mut lambda_m = CIE_WAVELENGTH_MIN_M;
    while lambda_m <= CIE_WAVELENGTH_MAX_M {
        let wavelength = Wavelength::new(lambda_m);
        let b_lambda = planck_spectral_radiance(wavelength, star_temp);
        let solar_irradiance = b_lambda * solid_angle_sun;

        if solar_irradiance > 0.0 && solar_irradiance.is_finite() {
            xyz_sun_toa = xyz_sun_toa + cmf_eval(wavelength) * solar_irradiance;

            let res_zenith = multiple_scattering_spectral_radiance(
                ray_origin,
                view_zenith,
                sun_dir_day,
                solar_irradiance,
                wavelength,
                atmosphere,
                ms_config,
            );
            xyz_zenith = xyz_zenith + cmf_eval(wavelength) * res_zenith.total_radiance;

            let res_horizon = multiple_scattering_spectral_radiance(
                ray_origin,
                view_horizon,
                s_refracted_day,
                solar_irradiance,
                wavelength,
                atmosphere,
                ms_config,
            );
            xyz_horizon = xyz_horizon + cmf_eval(wavelength) * res_horizon.total_radiance;

            let res_sunset = multiple_scattering_stellar_disk_spectral_radiance(
                ray_origin,
                view_sunset,
                sun_dir_sunset,
                star_angular_radius,
                solar_irradiance,
                wavelength,
                atmosphere,
                ms_config,
                16,
                0.6,
            );
            let sunset_radiance = res_sunset.total_radiance + b_lambda * res_sunset.transmittance;
            xyz_sunset = xyz_sunset + cmf_eval(wavelength) * sunset_radiance;
        }

        lambda_m += step;
    }

    xyz_sun_toa = xyz_sun_toa * step;
    xyz_zenith = xyz_zenith * step;
    xyz_horizon = xyz_horizon * step;
    xyz_sunset = xyz_sunset * step;

    IntegratedSpectralRadiances {
        xyz_zenith,
        xyz_horizon,
        xyz_sunset,
        xyz_sun_toa,
    }
}
