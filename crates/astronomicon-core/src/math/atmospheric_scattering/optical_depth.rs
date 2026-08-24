use crate::math::atmospheric_scattering::profiles::SphericalAtmosphere;
use crate::math::optics::{absorption_coefficient, rayleigh_scattering_coefficient};
use crate::units::{Length, Vector3, Wavelength};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SphericalOpticalDepth {
    pub gas_depth: f64,
    pub dust_depth: f64,
    pub cloud_depth: f64,
    pub volcanic_depth: f64,
}

impl SphericalOpticalDepth {
    pub fn new(gas_depth: f64, dust_depth: f64, cloud_depth: f64, volcanic_depth: f64) -> Self {
        Self {
            gas_depth: gas_depth.max(0.0),
            dust_depth: dust_depth.max(0.0),
            cloud_depth: cloud_depth.max(0.0),
            volcanic_depth: volcanic_depth.max(0.0),
        }
    }

    pub fn zero() -> Self {
        Self {
            gas_depth: 0.0,
            dust_depth: 0.0,
            cloud_depth: 0.0,
            volcanic_depth: 0.0,
        }
    }

    pub fn total_extinction_optical_depth(
        &self,
        wavelength: Wavelength,
        atmosphere: &SphericalAtmosphere,
    ) -> f64 {
        let beta_s_r0 = rayleigh_scattering_coefficient(
            wavelength,
            atmosphere.gas_optical_properties.refractivity_stp(),
            atmosphere.gas_optical_properties.king_factor(),
            atmosphere.surface_pressure,
            atmosphere.surface_temperature,
        );
        let beta_a_g0 = absorption_coefficient(
            &atmosphere.gas_optical_properties,
            wavelength,
            atmosphere.surface_pressure,
            atmosphere.surface_temperature,
        );
        let beta_e_r0 = beta_s_r0 + beta_a_g0;

        let k_e_dust = atmosphere
            .dust_profile
            .extinction_coefficient_at_wavelength(wavelength);
        let k_e_cloud = atmosphere
            .cloud_profile
            .extinction_coefficient_at_wavelength(wavelength);
        let k_e_volc = atmosphere
            .volcanic_profile
            .extinction_coefficient_at_wavelength(wavelength);

        beta_e_r0 * self.gas_depth
            + k_e_dust * self.dust_depth
            + k_e_cloud * self.cloud_depth
            + k_e_volc * self.volcanic_depth
    }
}

pub fn spherical_optical_depth_segment(
    start_pos: Vector3,
    end_pos: Vector3,
    atmosphere: &SphericalAtmosphere,
    samples: u32,
) -> SphericalOpticalDepth {
    let diff = end_pos - start_pos;
    let dist = diff.magnitude();

    if dist <= 0.0 || !dist.is_finite() {
        return SphericalOpticalDepth::zero();
    }

    let n = samples.max(2);
    let ds = dist / (n as f64);
    let dir = diff / dist;

    let r_p = atmosphere.planet_radius.value();
    let h_gas = atmosphere.gas_scale_height.value().max(1.0);

    let mut tau_gas = 0.0;
    let mut tau_dust = 0.0;
    let mut tau_cloud = 0.0;
    let mut tau_volc = 0.0;

    for i in 0..n {
        let s = ((i as f64) + 0.5) * ds;
        let p = start_pos + dir * s;
        let r = p.magnitude();
        let alt = (r - r_p).max(0.0);

        let exp_gas = -alt / h_gas;
        if exp_gas >= -700.0 {
            tau_gas += exp_gas.exp() * ds;
        }

        let alt_len = Length::new(alt);
        let rho_d = atmosphere.dust_profile.density_at_altitude(alt_len).value();
        let rho_c = atmosphere
            .cloud_profile
            .density_at_altitude(alt_len)
            .value();
        let rho_v = atmosphere
            .volcanic_profile
            .density_at_altitude(alt_len)
            .value();

        tau_dust += rho_d * ds;
        tau_cloud += rho_c * ds;
        tau_volc += rho_v * ds;
    }

    SphericalOpticalDepth::new(tau_gas, tau_dust, tau_cloud, tau_volc)
}

pub fn sun_path_optical_depth(
    sample_pos: Vector3,
    sun_dir: Vector3,
    atmosphere: &SphericalAtmosphere,
    samples: u32,
) -> Option<SphericalOpticalDepth> {
    let s = sun_dir.normalized();
    let r_p = atmosphere.planet_radius.value();
    let r_atm = atmosphere.atmosphere_top_radius.value();

    if r_p <= 0.0 || r_atm <= r_p {
        return None;
    }

    let r_sq = sample_pos.dot(&sample_pos);
    let b_p = sample_pos.dot(&s);
    let c_p = r_sq - r_p * r_p;
    let disc_p = b_p * b_p - c_p;

    if disc_p >= 0.0 {
        let sqrt_disc_p = disc_p.sqrt();
        let t_p1 = -b_p - sqrt_disc_p;
        let t_p2 = -b_p + sqrt_disc_p;

        if t_p1 > 1e-4 {
            return None;
        }
        if t_p2 > 1e-4 && b_p < 0.0 {
            return None;
        }
    }

    let b_atm = sample_pos.dot(&s);
    let c_atm = r_sq - r_atm * r_atm;
    let disc_atm = b_atm * b_atm - c_atm;

    if disc_atm < 0.0 {
        return Some(SphericalOpticalDepth::zero());
    }

    let t_exit = -b_atm + disc_atm.sqrt();
    if t_exit <= 1e-6 {
        return Some(SphericalOpticalDepth::zero());
    }

    let n = samples.max(2);
    let ds = t_exit / (n as f64);
    let h_gas = atmosphere.gas_scale_height.value().max(1.0);

    let mut tau_gas = 0.0;
    let mut tau_dust = 0.0;
    let mut tau_cloud = 0.0;
    let mut tau_volc = 0.0;

    for j in 0..n {
        let step = ((j as f64) + 0.5) * ds;
        let p_j = sample_pos + s * step;
        let r_j = p_j.magnitude();
        let alt_j = (r_j - r_p).max(0.0);

        let exp_gas = -alt_j / h_gas;
        if exp_gas >= -700.0 {
            tau_gas += exp_gas.exp() * ds;
        }

        let alt_len = Length::new(alt_j);
        let rho_d = atmosphere.dust_profile.density_at_altitude(alt_len).value();
        let rho_c = atmosphere
            .cloud_profile
            .density_at_altitude(alt_len)
            .value();
        let rho_v = atmosphere
            .volcanic_profile
            .density_at_altitude(alt_len)
            .value();

        tau_dust += rho_d * ds;
        tau_cloud += rho_c * ds;
        tau_volc += rho_v * ds;
    }

    Some(SphericalOpticalDepth::new(
        tau_gas, tau_dust, tau_cloud, tau_volc,
    ))
}
