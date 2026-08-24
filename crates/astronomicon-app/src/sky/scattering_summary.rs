use crate::sky::ScatteringCoefficients;
use astronomicon_core::chemistry::optics::GasOpticalProperties;
use astronomicon_core::math::atmospheric_scattering::{
    DustProfile, SphericalAtmosphere, VolcanicProfile, spherical_optical_depth_segment,
};
use astronomicon_core::math::optics::rayleigh_scattering_coefficient;
use astronomicon_core::units::{Length, Pressure, Temperature, Vector3, Wavelength};

pub struct ScatteringSummary {
    pub scattering: ScatteringCoefficients,
    pub total_optical_depth_r: f64,
    pub total_optical_depth_g: f64,
    pub total_optical_depth_b: f64,
}

pub fn resolve_scattering_summary(
    atmosphere: &SphericalAtmosphere,
    dust_profile: &DustProfile,
    volcanic_profile: &VolcanicProfile,
    opt_props: &GasOpticalProperties,
    surface_pressure: Pressure,
    surface_temperature: Temperature,
    ray_origin: Vector3,
) -> ScatteringSummary {
    let wl_r = Wavelength::new(680e-9);
    let wl_g = Wavelength::new(550e-9);
    let wl_b = Wavelength::new(440e-9);

    let p_surf = surface_pressure;
    let refr_stp = opt_props.refractivity_stp();
    let king = opt_props.king_factor();

    let b_r_r = rayleigh_scattering_coefficient(wl_r, refr_stp, king, p_surf, surface_temperature);
    let b_r_g = rayleigh_scattering_coefficient(wl_g, refr_stp, king, p_surf, surface_temperature);
    let b_r_b = rayleigh_scattering_coefficient(wl_b, refr_stp, king, p_surf, surface_temperature);

    let b_m_r = dust_profile.density_at_altitude(Length::new(0.0)).value()
        * dust_profile.scattering_coefficient_at_wavelength(wl_r)
        + volcanic_profile
            .density_at_altitude(Length::new(0.0))
            .value()
            * volcanic_profile.scattering_coefficient_at_wavelength(wl_r);

    let b_m_g = dust_profile.density_at_altitude(Length::new(0.0)).value()
        * dust_profile.scattering_coefficient_at_wavelength(wl_g)
        + volcanic_profile
            .density_at_altitude(Length::new(0.0))
            .value()
            * volcanic_profile.scattering_coefficient_at_wavelength(wl_g);

    let b_m_b = dust_profile.density_at_altitude(Length::new(0.0)).value()
        * dust_profile.scattering_coefficient_at_wavelength(wl_b)
        + volcanic_profile
            .density_at_altitude(Length::new(0.0))
            .value()
            * volcanic_profile.scattering_coefficient_at_wavelength(wl_b);

    let scattering = ScatteringCoefficients {
        rayleigh_r: b_r_r,
        rayleigh_g: b_r_g,
        rayleigh_b: b_r_b,
        mie_r: b_m_r,
        mie_g: b_m_g,
        mie_b: b_m_b,
    };

    let vertical_top = Vector3::new(0.0, atmosphere.atmosphere_top_radius.value(), 0.0);
    let vertical_depth = spherical_optical_depth_segment(ray_origin, vertical_top, atmosphere, 64);

    let tau_tot_r = vertical_depth.total_extinction_optical_depth(wl_r, atmosphere);
    let tau_tot_g = vertical_depth.total_extinction_optical_depth(wl_g, atmosphere);
    let tau_tot_b = vertical_depth.total_extinction_optical_depth(wl_b, atmosphere);

    ScatteringSummary {
        scattering,
        total_optical_depth_r: tau_tot_r,
        total_optical_depth_g: tau_tot_g,
        total_optical_depth_b: tau_tot_b,
    }
}
