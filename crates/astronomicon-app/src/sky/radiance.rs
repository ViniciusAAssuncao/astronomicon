use crate::sky::optical_column::{
    SpectralOpticalDepth, wavelength_blue, wavelength_green, wavelength_red,
};
use astronomicon_core::math::optics::{
    delta_eddington_two_stream, henyey_greenstein_phase_function, rayleigh_phase_function,
    relative_airmass,
};
use astronomicon_core::math::radiation::planck_spectral_radiance;
use astronomicon_core::units::{Angle, Irradiance, Temperature};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CanonicalGeometry {
    pub sun_zenith_angle: Angle,
    pub view_zenith_angle: Angle,
    pub scattering_angle: Angle,
}

impl CanonicalGeometry {
    pub fn zenith() -> Self {
        Self {
            sun_zenith_angle: Angle::new(0.0),
            view_zenith_angle: Angle::new(0.0),
            scattering_angle: Angle::new(0.0),
        }
    }

    pub fn horizon() -> Self {
        let sun_zenith = Angle::new(PI / 3.0);
        let view_zenith = Angle::new(88.0 * PI / 180.0);
        let cos_theta = (PI / 3.0).cos() * (88.0 * PI / 180.0).cos();
        let scattering = Angle::new(cos_theta.clamp(-1.0, 1.0).acos());
        Self {
            sun_zenith_angle: sun_zenith,
            view_zenith_angle: view_zenith,
            scattering_angle: scattering,
        }
    }

    pub fn sunset() -> Self {
        let sun_zenith = Angle::new(88.0 * PI / 180.0);
        let view_zenith = Angle::new(85.0 * PI / 180.0);
        let az_diff = 15.0 * PI / 180.0;
        let cos_theta = (88.0 * PI / 180.0).cos() * (85.0 * PI / 180.0).cos()
            + (88.0 * PI / 180.0).sin() * (85.0 * PI / 180.0).sin() * az_diff.cos();
        let scattering = Angle::new(cos_theta.clamp(-1.0, 1.0).acos());
        Self {
            sun_zenith_angle: sun_zenith,
            view_zenith_angle: view_zenith,
            scattering_angle: scattering,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpectralRadiance {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl SpectralRadiance {
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b }
    }

    pub fn as_tuple(&self) -> (f64, f64, f64) {
        (self.r, self.g, self.b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeometryRadianceBreakdown {
    pub single_scattering: SpectralRadiance,
    pub diffuse: SpectralRadiance,
    pub total: SpectralRadiance,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyRadianceDiagnostic {
    pub solar_irradiance: SpectralRadiance,
    pub zenith_radiance: SpectralRadiance,
    pub horizon_radiance: SpectralRadiance,
    pub sunset_radiance: SpectralRadiance,
    pub zenith_diffuse: SpectralRadiance,
    pub zenith_single: SpectralRadiance,
    pub horizon_diffuse: SpectralRadiance,
    pub horizon_single: SpectralRadiance,
    pub sunset_diffuse: SpectralRadiance,
    pub sunset_single: SpectralRadiance,
}

pub fn resolve_spectral_solar_irradiance(
    total_irradiance: Irradiance,
    effective_temperature: Temperature,
) -> SpectralRadiance {
    let f_total = total_irradiance.value().max(0.0);
    if f_total <= 0.0 {
        return SpectralRadiance::new(0.0, 0.0, 0.0);
    }

    let b_r = planck_spectral_radiance(wavelength_red(), effective_temperature);
    let b_g = planck_spectral_radiance(wavelength_green(), effective_temperature);
    let b_b = planck_spectral_radiance(wavelength_blue(), effective_temperature);
    let b_sum = b_r + b_g + b_b;

    if b_sum <= 0.0 || !b_sum.is_finite() {
        return SpectralRadiance::new(f_total, f_total, f_total);
    }

    let b_mean = b_sum / 3.0;
    let scale = f_total / b_mean;
    SpectralRadiance::new(b_r * scale, b_g * scale, b_b * scale)
}

pub fn calculate_radiance_breakdown_at_geometry(
    optical_column: &SpectralOpticalDepth,
    solar_irradiance: SpectralRadiance,
    geometry: CanonicalGeometry,
    surface_albedo: f64,
) -> GeometryRadianceBreakdown {
    let m_s = relative_airmass(geometry.sun_zenith_angle);
    let m_v = relative_airmass(geometry.view_zenith_angle);
    let theta = geometry.scattering_angle;
    let mu0 = geometry.sun_zenith_angle.value().cos().max(0.0001);

    let p_r = rayleigh_phase_function(theta);

    let taus_ext = [
        optical_column.total_r,
        optical_column.total_g,
        optical_column.total_b,
    ];
    let taus_rayleigh = [
        optical_column.rayleigh_r,
        optical_column.rayleigh_g,
        optical_column.rayleigh_b,
    ];
    let taus_aero_ext = [
        optical_column.aerosol_r,
        optical_column.aerosol_g,
        optical_column.aerosol_b,
    ];
    let ssas = [
        optical_column.single_scattering_albedo_r,
        optical_column.single_scattering_albedo_g,
        optical_column.single_scattering_albedo_b,
    ];
    let gs = [
        optical_column.asymmetry_factor_r,
        optical_column.asymmetry_factor_g,
        optical_column.asymmetry_factor_b,
    ];
    let f_sols = [
        solar_irradiance.r,
        solar_irradiance.g,
        solar_irradiance.b,
    ];

    let mut single_rad = [0.0; 3];
    let mut diffuse_rad = [0.0; 3];
    let mut total_rad = [0.0; 3];

    for i in 0..3 {
        let tau = taus_ext[i].max(0.0);
        let tau_r = taus_rayleigh[i].max(0.0);
        let tau_a = taus_aero_ext[i].max(0.0);
        let omega = ssas[i].clamp(0.0, 1.0);
        let g = gs[i].clamp(-0.999, 0.999);
        let f_0 = f_sols[i].max(0.0);

        if f_0 <= 0.0 || tau <= 0.0 || omega <= 0.0 {
            single_rad[i] = 0.0;
            diffuse_rad[i] = 0.0;
            total_rad[i] = 0.0;
            continue;
        }

        let p_hg = henyey_greenstein_phase_function(theta, g);
        let tau_sca_tot = tau_r + tau_a;
        let phase = if tau_sca_tot > 1e-12 {
            (tau_r * p_r + tau_a * p_hg) / tau_sca_tot
        } else {
            p_r
        };

        let t_geom = if (m_s - m_v).abs() < 1e-4 {
            m_v * tau * (-m_v * tau).exp()
        } else {
            (m_v / (m_s - m_v)) * ((-m_v * tau).exp() - (-m_s * tau).exp())
        };

        let rad_s = f_0 * omega * phase * t_geom.max(0.0);
        single_rad[i] = if rad_s.is_finite() && rad_s > 0.0 {
            rad_s
        } else {
            0.0
        };

        let two_stream = delta_eddington_two_stream(tau, omega, g, mu0, surface_albedo);
        let rad_d = (two_stream.transmittance * mu0 * f_0) / PI;
        diffuse_rad[i] = if rad_d.is_finite() && rad_d > 0.0 {
            rad_d
        } else {
            0.0
        };

        let tot = single_rad[i] + diffuse_rad[i];
        total_rad[i] = if tot.is_finite() && tot > 0.0 {
            tot
        } else {
            0.0
        };
    }

    GeometryRadianceBreakdown {
        single_scattering: SpectralRadiance::new(single_rad[0], single_rad[1], single_rad[2]),
        diffuse: SpectralRadiance::new(diffuse_rad[0], diffuse_rad[1], diffuse_rad[2]),
        total: SpectralRadiance::new(total_rad[0], total_rad[1], total_rad[2]),
    }
}

pub fn calculate_single_scattering_radiance(
    optical_column: &SpectralOpticalDepth,
    solar_irradiance: SpectralRadiance,
    geometry: CanonicalGeometry,
) -> SpectralRadiance {
    calculate_radiance_breakdown_at_geometry(optical_column, solar_irradiance, geometry, 0.0)
        .single_scattering
}

pub fn calculate_diffuse_radiance_at_geometry(
    optical_column: &SpectralOpticalDepth,
    solar_irradiance: SpectralRadiance,
    geometry: CanonicalGeometry,
    surface_albedo: f64,
) -> SpectralRadiance {
    calculate_radiance_breakdown_at_geometry(
        optical_column,
        solar_irradiance,
        geometry,
        surface_albedo,
    )
    .diffuse
}

pub fn calculate_radiance_at_geometry(
    optical_column: &SpectralOpticalDepth,
    solar_irradiance: SpectralRadiance,
    geometry: CanonicalGeometry,
    surface_albedo: f64,
) -> SpectralRadiance {
    calculate_radiance_breakdown_at_geometry(
        optical_column,
        solar_irradiance,
        geometry,
        surface_albedo,
    )
    .total
}

pub fn calculate_sky_radiances(
    optical_column: &SpectralOpticalDepth,
    solar_irradiance: SpectralRadiance,
    surface_albedo: f64,
) -> SkyRadianceDiagnostic {
    let zenith = calculate_radiance_breakdown_at_geometry(
        optical_column,
        solar_irradiance,
        CanonicalGeometry::zenith(),
        surface_albedo,
    );
    let horizon = calculate_radiance_breakdown_at_geometry(
        optical_column,
        solar_irradiance,
        CanonicalGeometry::horizon(),
        surface_albedo,
    );
    let sunset = calculate_radiance_breakdown_at_geometry(
        optical_column,
        solar_irradiance,
        CanonicalGeometry::sunset(),
        surface_albedo,
    );

    SkyRadianceDiagnostic {
        solar_irradiance,
        zenith_radiance: zenith.total,
        horizon_radiance: horizon.total,
        sunset_radiance: sunset.total,
        zenith_diffuse: zenith.diffuse,
        zenith_single: zenith.single_scattering,
        horizon_diffuse: horizon.diffuse,
        horizon_single: horizon.single_scattering,
        sunset_diffuse: sunset.diffuse,
        sunset_single: sunset.single_scattering,
    }
}
