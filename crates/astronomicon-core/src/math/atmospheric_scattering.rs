use crate::chemistry::optics::GasOpticalProperties;
use crate::math::aerosol::refractivity_at_temperature_pressure;
use crate::math::colorimetry::{
    cie_color_matching_functions,
    exposure_tone_map,
    linear_to_srgb_gamma,
    xyz_to_linear_srgb,
    ColorXYZ,
};
use crate::math::optics::{
    absorption_coefficient,
    henyey_greenstein_phase_function,
    rayleigh_phase_function_with_depolarization,
    rayleigh_scattering_coefficient,
    refracted_sun_direction,
    unrefracted_sun_direction,
};
use crate::math::radiation::planck_spectral_radiance;
use crate::math::radiometry::{stellar_disk_sample_directions, stellar_limb_darkening};
use crate::units::constants::{
    CIE_WAVELENGTH_MAX_M,
    CIE_WAVELENGTH_MIN_M,
    CIE_WAVELENGTH_STEP_M,
    OPTICAL_REFERENCE_WAVELENGTH,
};
use crate::units::{
    Angle,
    ColorRGB,
    Density,
    Length,
    Pressure,
    Temperature,
    Vector3,
    Wavelength,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericRaymarchConfig {
    pub view_samples: u32,
    pub sun_samples: u32,
    pub atmosphere_top_altitude: Length,
}

impl AtmosphericRaymarchConfig {
    pub fn new(view_samples: u32, sun_samples: u32, atmosphere_top_altitude: Length) -> Self {
        Self {
            view_samples: view_samples.max(4),
            sun_samples: sun_samples.max(2),
            atmosphere_top_altitude,
        }
    }

    pub fn fast() -> Self {
        Self {
            view_samples: 16,
            sun_samples: 8,
            atmosphere_top_altitude: Length::new(100_000.0),
        }
    }

    pub fn accurate() -> Self {
        Self {
            view_samples: 64,
            sun_samples: 32,
            atmosphere_top_altitude: Length::new(100_000.0),
        }
    }
}

impl Default for AtmosphericRaymarchConfig {
    fn default() -> Self {
        Self {
            view_samples: 32,
            sun_samples: 16,
            atmosphere_top_altitude: Length::new(100_000.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DustProfile {
    pub surface_density: Density,
    pub scale_height: Length,
    pub particle_radius: Length,
    pub particle_density: Density,
    pub refractive_index_real: f64,
    pub refractive_index_imag: f64,
    pub asymmetry_factor_g: f64,
    pub mass_extinction_coeff: f64,
    pub mass_scattering_coeff: f64,
    pub angstrom_exponent: f64,
}

impl DustProfile {
    pub fn new(
        surface_density: Density,
        scale_height: Length,
        particle_radius: Length,
        particle_density: Density,
        refractive_index_real: f64,
        refractive_index_imag: f64,
        asymmetry_factor_g: f64,
        mass_extinction_coeff: f64,
        mass_scattering_coeff: f64,
        angstrom_exponent: f64,
    ) -> Self {
        Self {
            surface_density,
            scale_height,
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            asymmetry_factor_g,
            mass_extinction_coeff,
            mass_scattering_coeff,
            angstrom_exponent,
        }
    }

    pub fn from_material(
        surface_density: Density,
        scale_height: Length,
        particle_radius: Length,
        particle_density: Density,
        refractive_index_real: f64,
        refractive_index_imag: f64,
    ) -> Self {
        let (ke, ks, _, g, alpha) = crate::math::optics::mass_optical_efficiencies(
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH),
        );
        Self {
            surface_density,
            scale_height,
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            asymmetry_factor_g: g.clamp(-0.999, 0.999),
            mass_extinction_coeff: ke.max(0.0),
            mass_scattering_coeff: ks.max(0.0),
            angstrom_exponent: alpha.clamp(0.0, 4.0),
        }
    }

    pub fn zero() -> Self {
        Self {
            surface_density: Density::new(0.0),
            scale_height: Length::new(1000.0),
            particle_radius: Length::new(1.0e-6),
            particle_density: Density::new(2650.0),
            refractive_index_real: 1.55,
            refractive_index_imag: 0.005,
            asymmetry_factor_g: 0.7,
            mass_extinction_coeff: 0.0,
            mass_scattering_coeff: 0.0,
            angstrom_exponent: 1.0,
        }
    }

    pub fn density_at_altitude(&self, altitude: Length) -> Density {
        let z = altitude.value();
        let h = self.scale_height.value();
        let rho0 = self.surface_density.value();
        if z < 0.0 || rho0 <= 0.0 || h <= 0.0 || !z.is_finite() || !rho0.is_finite() || !h.is_finite() {
            return Density::new(0.0);
        }
        let exponent = -z / h;
        if exponent < -700.0 {
            Density::new(0.0)
        } else {
            Density::new(rho0 * exponent.exp())
        }
    }

    pub fn integrated_column_between(&self, z_start: Length, z_end: Length) -> f64 {
        let z0 = z_start.value().max(0.0);
        let z1 = z_end.value().max(0.0);
        let h = self.scale_height.value();
        let rho0 = self.surface_density.value();
        if rho0 <= 0.0 || h <= 0.0 || !rho0.is_finite() || !h.is_finite() || z0 >= z1 {
            return 0.0;
        }
        let exp0 = (-z0 / h).exp();
        let exp1 = (-z1 / h).exp();
        rho0 * h * (exp0 - exp1).max(0.0)
    }

    pub fn scattering_coefficient_at_wavelength(&self, wavelength: Wavelength) -> f64 {
        let lambda = wavelength.value();
        let lambda0 = OPTICAL_REFERENCE_WAVELENGTH;
        if lambda <= 0.0 || !lambda.is_finite() || self.mass_scattering_coeff <= 0.0 {
            return 0.0;
        }
        self.mass_scattering_coeff * (lambda0 / lambda).powf(self.angstrom_exponent)
    }

    pub fn extinction_coefficient_at_wavelength(&self, wavelength: Wavelength) -> f64 {
        let lambda = wavelength.value();
        let lambda0 = OPTICAL_REFERENCE_WAVELENGTH;
        if lambda <= 0.0 || !lambda.is_finite() || self.mass_extinction_coeff <= 0.0 {
            return 0.0;
        }
        let sca = self.scattering_coefficient_at_wavelength(wavelength);
        let ext = self.mass_extinction_coeff * (lambda0 / lambda).powf(self.angstrom_exponent);
        ext.max(sca)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudProfile {
    pub base_density: Density,
    pub coverage_fraction: f64,
    pub lcl_altitude: Length,
    pub cloud_top_altitude: Length,
    pub particle_radius: Length,
    pub particle_density: Density,
    pub refractive_index_real: f64,
    pub refractive_index_imag: f64,
    pub asymmetry_factor_g: f64,
    pub mass_extinction_coeff: f64,
    pub mass_scattering_coeff: f64,
    pub angstrom_exponent: f64,
}

impl CloudProfile {
    pub fn new(
        base_density: Density,
        coverage_fraction: f64,
        lcl_altitude: Length,
        cloud_top_altitude: Length,
        particle_radius: Length,
        particle_density: Density,
        refractive_index_real: f64,
        refractive_index_imag: f64,
        asymmetry_factor_g: f64,
        mass_extinction_coeff: f64,
        mass_scattering_coeff: f64,
        angstrom_exponent: f64,
    ) -> Self {
        Self {
            base_density,
            coverage_fraction: coverage_fraction.clamp(0.0, 1.0),
            lcl_altitude,
            cloud_top_altitude,
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            asymmetry_factor_g,
            mass_extinction_coeff,
            mass_scattering_coeff,
            angstrom_exponent,
        }
    }

    pub fn from_material(
        base_density: Density,
        coverage_fraction: f64,
        lcl_altitude: Length,
        cloud_top_altitude: Length,
        particle_radius: Length,
        particle_density: Density,
        refractive_index_real: f64,
        refractive_index_imag: f64,
    ) -> Self {
        let (ke, ks, _, g, alpha) = crate::math::optics::mass_optical_efficiencies(
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH),
        );
        Self {
            base_density,
            coverage_fraction: coverage_fraction.clamp(0.0, 1.0),
            lcl_altitude,
            cloud_top_altitude,
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            asymmetry_factor_g: g.clamp(-0.999, 0.999),
            mass_extinction_coeff: ke.max(0.0),
            mass_scattering_coeff: ks.max(0.0),
            angstrom_exponent: alpha.clamp(0.0, 4.0),
        }
    }

    pub fn zero() -> Self {
        Self {
            base_density: Density::new(0.0),
            coverage_fraction: 0.0,
            lcl_altitude: Length::new(1000.0),
            cloud_top_altitude: Length::new(4000.0),
            particle_radius: Length::new(10.0e-6),
            particle_density: Density::new(1000.0),
            refractive_index_real: 1.333,
            refractive_index_imag: 1.0e-8,
            asymmetry_factor_g: 0.85,
            mass_extinction_coeff: 0.0,
            mass_scattering_coeff: 0.0,
            angstrom_exponent: 0.1,
        }
    }

    pub fn density_at_altitude(&self, altitude: Length) -> Density {
        let z = altitude.value();
        let z_lcl = self.lcl_altitude.value();
        let z_top = self.cloud_top_altitude.value();
        let cov = self.coverage_fraction.clamp(0.0, 1.0);
        let rho_base = self.base_density.value();

        if z < z_lcl || z > z_top || z_top <= z_lcl || cov <= 0.0 || rho_base <= 0.0 || !z.is_finite() {
            return Density::new(0.0);
        }

        let dz = z_top - z_lcl;
        let shape = 4.0 * (z - z_lcl) * (z_top - z) / (dz * dz);
        let rho = rho_base * cov * shape.clamp(0.0, 1.0);
        Density::new(rho)
    }

    pub fn integrated_column_between(&self, z_start: Length, z_end: Length) -> f64 {
        let z0 = z_start.value().max(0.0);
        let z1 = z_end.value().max(0.0);
        let z_lcl = self.lcl_altitude.value();
        let z_top = self.cloud_top_altitude.value();
        let cov = self.coverage_fraction.clamp(0.0, 1.0);
        let rho_base = self.base_density.value();

        if z0 >= z1 || z_top <= z_lcl || cov <= 0.0 || rho_base <= 0.0 {
            return 0.0;
        }

        let a = z0.max(z_lcl).min(z_top);
        let b = z1.max(z_lcl).min(z_top);
        if a >= b {
            return 0.0;
        }

        let dz = z_top - z_lcl;
        let int_fn = |z_val: f64| -> f64 {
            let u = (z_val - z_lcl) / dz;
            dz * (2.0 * u * u - (4.0 / 3.0) * u * u * u)
        };

        let integral = int_fn(b) - int_fn(a);
        rho_base * cov * integral.max(0.0)
    }

    pub fn scattering_coefficient_at_wavelength(&self, wavelength: Wavelength) -> f64 {
        let lambda = wavelength.value();
        let lambda0 = OPTICAL_REFERENCE_WAVELENGTH;
        if lambda <= 0.0 || !lambda.is_finite() || self.mass_scattering_coeff <= 0.0 {
            return 0.0;
        }
        self.mass_scattering_coeff * (lambda0 / lambda).powf(self.angstrom_exponent)
    }

    pub fn extinction_coefficient_at_wavelength(&self, wavelength: Wavelength) -> f64 {
        let lambda = wavelength.value();
        let lambda0 = OPTICAL_REFERENCE_WAVELENGTH;
        if lambda <= 0.0 || !lambda.is_finite() || self.mass_extinction_coeff <= 0.0 {
            return 0.0;
        }
        let sca = self.scattering_coefficient_at_wavelength(wavelength);
        let ext = self.mass_extinction_coeff * (lambda0 / lambda).powf(self.angstrom_exponent);
        ext.max(sca)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolcanicProfile {
    pub injection_altitude: Length,
    pub plume_thickness: Length,
    pub peak_density: Density,
    pub particle_radius: Length,
    pub particle_density: Density,
    pub refractive_index_real: f64,
    pub refractive_index_imag: f64,
    pub asymmetry_factor_g: f64,
    pub mass_extinction_coeff: f64,
    pub mass_scattering_coeff: f64,
    pub angstrom_exponent: f64,
}

impl VolcanicProfile {
    pub fn new(
        injection_altitude: Length,
        plume_thickness: Length,
        peak_density: Density,
        particle_radius: Length,
        particle_density: Density,
        refractive_index_real: f64,
        refractive_index_imag: f64,
        asymmetry_factor_g: f64,
        mass_extinction_coeff: f64,
        mass_scattering_coeff: f64,
        angstrom_exponent: f64,
    ) -> Self {
        Self {
            injection_altitude,
            plume_thickness,
            peak_density,
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            asymmetry_factor_g,
            mass_extinction_coeff,
            mass_scattering_coeff,
            angstrom_exponent,
        }
    }

    pub fn from_material(
        injection_altitude: Length,
        plume_thickness: Length,
        peak_density: Density,
        particle_radius: Length,
        particle_density: Density,
        refractive_index_real: f64,
        refractive_index_imag: f64,
    ) -> Self {
        let (ke, ks, _, g, alpha) = crate::math::optics::mass_optical_efficiencies(
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH),
        );
        Self {
            injection_altitude,
            plume_thickness,
            peak_density,
            particle_radius,
            particle_density,
            refractive_index_real,
            refractive_index_imag,
            asymmetry_factor_g: g.clamp(-0.999, 0.999),
            mass_extinction_coeff: ke.max(0.0),
            mass_scattering_coeff: ks.max(0.0),
            angstrom_exponent: alpha.clamp(0.0, 4.0),
        }
    }

    pub fn zero() -> Self {
        Self {
            injection_altitude: Length::new(0.0),
            plume_thickness: Length::new(1000.0),
            peak_density: Density::new(0.0),
            particle_radius: Length::new(5.0e-6),
            particle_density: Density::new(2400.0),
            refractive_index_real: 1.52,
            refractive_index_imag: 0.015,
            asymmetry_factor_g: 0.75,
            mass_extinction_coeff: 0.0,
            mass_scattering_coeff: 0.0,
            angstrom_exponent: 1.2,
        }
    }

    pub fn density_at_altitude(&self, altitude: Length) -> Density {
        let z = altitude.value();
        let z_inj = self.injection_altitude.value();
        let h_plume = self.plume_thickness.value();
        let rho_peak = self.peak_density.value();

        if rho_peak <= 0.0 || h_plume <= 0.0 || !z.is_finite() || !rho_peak.is_finite() || !h_plume.is_finite() {
            return Density::new(0.0);
        }

        if z_inj <= 0.0 {
            let exponent = -z / h_plume;
            if exponent < -700.0 {
                Density::new(0.0)
            } else {
                Density::new(rho_peak * exponent.exp())
            }
        } else {
            let dz = (z - z_inj) / h_plume;
            let exponent = -0.5 * dz * dz;
            if exponent < -700.0 {
                Density::new(0.0)
            } else {
                Density::new(rho_peak * exponent.exp())
            }
        }
    }

    pub fn scattering_coefficient_at_wavelength(&self, wavelength: Wavelength) -> f64 {
        let lambda = wavelength.value();
        let lambda0 = OPTICAL_REFERENCE_WAVELENGTH;
        if lambda <= 0.0 || !lambda.is_finite() || self.mass_scattering_coeff <= 0.0 {
            return 0.0;
        }
        self.mass_scattering_coeff * (lambda0 / lambda).powf(self.angstrom_exponent)
    }

    pub fn extinction_coefficient_at_wavelength(&self, wavelength: Wavelength) -> f64 {
        let lambda = wavelength.value();
        let lambda0 = OPTICAL_REFERENCE_WAVELENGTH;
        if lambda <= 0.0 || !lambda.is_finite() || self.mass_extinction_coeff <= 0.0 {
            return 0.0;
        }
        let sca = self.scattering_coefficient_at_wavelength(wavelength);
        let ext = self.mass_extinction_coeff * (lambda0 / lambda).powf(self.angstrom_exponent);
        ext.max(sca)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SphericalAtmosphere {
    pub planet_radius: Length,
    pub atmosphere_top_radius: Length,
    pub surface_pressure: Pressure,
    pub surface_temperature: Temperature,
    pub gas_scale_height: Length,
    pub gas_optical_properties: GasOpticalProperties,
    pub dust_profile: DustProfile,
    pub cloud_profile: CloudProfile,
    pub volcanic_profile: VolcanicProfile,
}

impl SphericalAtmosphere {
    pub fn new(
        planet_radius: Length,
        atmosphere_top_altitude: Length,
        surface_pressure: Pressure,
        surface_temperature: Temperature,
        gas_scale_height: Length,
        gas_optical_properties: GasOpticalProperties,
        dust_profile: DustProfile,
        cloud_profile: CloudProfile,
        volcanic_profile: VolcanicProfile,
    ) -> Self {
        let top_r = Length::new(
            planet_radius.value() + atmosphere_top_altitude.value().max(1000.0),
        );
        Self {
            planet_radius,
            atmosphere_top_radius: top_r,
            surface_pressure,
            surface_temperature,
            gas_scale_height,
            gas_optical_properties,
            dust_profile,
            cloud_profile,
            volcanic_profile,
        }
    }
}

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

        let k_e_dust = atmosphere.dust_profile.extinction_coefficient_at_wavelength(wavelength);
        let k_e_cloud = atmosphere.cloud_profile.extinction_coefficient_at_wavelength(wavelength);
        let k_e_volc = atmosphere.volcanic_profile.extinction_coefficient_at_wavelength(wavelength);

        beta_e_r0 * self.gas_depth
            + k_e_dust * self.dust_depth
            + k_e_cloud * self.cloud_depth
            + k_e_volc * self.volcanic_depth
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SphericalScatteringResult {
    pub in_scattered_radiance: f64,
    pub optical_depth: f64,
    pub transmittance: f64,
}

pub fn ray_sphere_intersections(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sphere_radius: Length,
) -> Option<(Length, Length)> {
    let r = sphere_radius.value();
    let d = ray_dir.normalized();

    if r <= 0.0 || !r.is_finite() {
        return None;
    }

    let b = ray_origin.dot(&d);
    let c = ray_origin.dot(&ray_origin) - r * r;
    let disc = b * b - c;

    if disc < 0.0 {
        return None;
    }

    let sqrt_disc = disc.sqrt();
    let t1 = -b - sqrt_disc;
    let t2 = -b + sqrt_disc;

    Some((Length::new(t1), Length::new(t2)))
}

pub fn ray_atmosphere_segment(
    ray_origin: Vector3,
    ray_dir: Vector3,
    planet_radius: Length,
    atmosphere_top_radius: Length,
) -> Option<(Length, Length, bool)> {
    let r_p = planet_radius.value();
    let r_atm = atmosphere_top_radius.value();
    let d = ray_dir.normalized();

    if r_p <= 0.0 || r_atm <= r_p || !r_p.is_finite() || !r_atm.is_finite() {
        return None;
    }

    let r0_sq = ray_origin.dot(&ray_origin);
    let r0 = r0_sq.sqrt();

    let b_atm = ray_origin.dot(&d);
    let c_atm = r0_sq - r_atm * r_atm;
    let disc_atm = b_atm * b_atm - c_atm;

    if disc_atm < 0.0 {
        return None;
    }

    let sqrt_disc_atm = disc_atm.sqrt();
    let t_atm1 = -b_atm - sqrt_disc_atm;
    let t_atm2 = -b_atm + sqrt_disc_atm;

    if t_atm2 <= 0.0 {
        return None;
    }

    let t_start = t_atm1.max(0.0);
    let mut t_end = t_atm2;
    let mut hits_ground = false;

    let b_p = ray_origin.dot(&d);
    let c_p = r0_sq - r_p * r_p;
    let disc_p = b_p * b_p - c_p;

    if disc_p >= 0.0 {
        let sqrt_disc_p = disc_p.sqrt();
        let t_p1 = -b_p - sqrt_disc_p;
        let t_p2 = -b_p + sqrt_disc_p;

        if t_p1 > 1e-6 {
            if t_p1 < t_end {
                t_end = t_p1;
                hits_ground = true;
            }
        } else if t_p2 > 1e-6 && b_p < 0.0 {
            if r0 <= r_p + 1.0 {
                return None;
            }
            t_end = 0.0;
            hits_ground = true;
        }
    }

    if t_end <= t_start + 1e-6 {
        return None;
    }

    Some((Length::new(t_start), Length::new(t_end), hits_ground))
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
        let rho_c = atmosphere.cloud_profile.density_at_altitude(alt_len).value();
        let rho_v = atmosphere.volcanic_profile.density_at_altitude(alt_len).value();

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
        let rho_c = atmosphere.cloud_profile.density_at_altitude(alt_len).value();
        let rho_v = atmosphere.volcanic_profile.density_at_altitude(alt_len).value();

        tau_dust += rho_d * ds;
        tau_cloud += rho_c * ds;
        tau_volc += rho_v * ds;
    }

    Some(SphericalOpticalDepth::new(tau_gas, tau_dust, tau_cloud, tau_volc))
}

pub fn single_scattering_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> SphericalScatteringResult {
    if solar_spectral_irradiance <= 0.0 || !solar_spectral_irradiance.is_finite() {
        return SphericalScatteringResult {
            in_scattered_radiance: 0.0,
            optical_depth: 0.0,
            transmittance: 1.0,
        };
    }

    let v_dir = ray_dir.normalized();
    let s_dir = sun_dir.normalized();

    let segment = ray_atmosphere_segment(
        ray_origin,
        v_dir,
        atmosphere.planet_radius,
        atmosphere.atmosphere_top_radius,
    );

    let (t_start, t_end, _hits_ground) = match segment {
        Some(seg) => seg,
        None => {
            return SphericalScatteringResult {
                in_scattered_radiance: 0.0,
                optical_depth: 0.0,
                transmittance: 1.0,
            };
        }
    };

    let total_path_len = t_end.value() - t_start.value();
    if total_path_len <= 0.0 || !total_path_len.is_finite() {
        return SphericalScatteringResult {
            in_scattered_radiance: 0.0,
            optical_depth: 0.0,
            transmittance: 1.0,
        };
    }

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

    let k_s_dust = atmosphere.dust_profile.scattering_coefficient_at_wavelength(wavelength);
    let k_e_dust = atmosphere.dust_profile.extinction_coefficient_at_wavelength(wavelength);
    let g_dust = atmosphere.dust_profile.asymmetry_factor_g;

    let k_s_cloud = atmosphere.cloud_profile.scattering_coefficient_at_wavelength(wavelength);
    let k_e_cloud = atmosphere.cloud_profile.extinction_coefficient_at_wavelength(wavelength);
    let g_cloud = atmosphere.cloud_profile.asymmetry_factor_g;

    let k_s_volc = atmosphere.volcanic_profile.scattering_coefficient_at_wavelength(wavelength);
    let k_e_volc = atmosphere.volcanic_profile.extinction_coefficient_at_wavelength(wavelength);
    let g_volc = atmosphere.volcanic_profile.asymmetry_factor_g;

    let cos_theta = v_dir.dot(&s_dir).clamp(-1.0, 1.0);
    let theta = Angle::new(cos_theta.acos());
    let phase_r = rayleigh_phase_function_with_depolarization(
        theta,
        atmosphere.gas_optical_properties.king_factor(),
    );

    let n_view = config.view_samples.max(4);
    let ds = total_path_len / (n_view as f64);

    let mut tau_view_gas = 0.0;
    let mut tau_view_dust = 0.0;
    let mut tau_view_cloud = 0.0;
    let mut tau_view_volc = 0.0;
    let mut in_scatter_accum = 0.0;

    let r_p = atmosphere.planet_radius.value();
    let r_top = atmosphere.atmosphere_top_radius.value();
    let h_gas = atmosphere.gas_scale_height.value().max(1.0);

    for i in 0..n_view {
        let s_i = t_start.value() + ((i as f64) + 0.5) * ds;
        let pos_i = ray_origin + v_dir * s_i;
        let r_i = pos_i.magnitude();
        let alt_i = (r_i - r_p).max(0.0);

        if alt_i > r_top - r_p {
            continue;
        }

        let exp_gas = -alt_i / h_gas;
        let rho_g = if exp_gas >= -700.0 { exp_gas.exp() } else { 0.0 };

        let alt_len = Length::new(alt_i);
        let rho_d = atmosphere.dust_profile.density_at_altitude(alt_len).value();
        let rho_c = atmosphere.cloud_profile.density_at_altitude(alt_len).value();
        let rho_v = atmosphere.volcanic_profile.density_at_altitude(alt_len).value();

        tau_view_gas += rho_g * ds;
        tau_view_dust += rho_d * ds;
        tau_view_cloud += rho_c * ds;
        tau_view_volc += rho_v * ds;

        let b_s_d = rho_d * k_s_dust;
        let b_s_c = rho_c * k_s_cloud;
        let b_s_v = rho_v * k_s_volc;
        let b_s_aero = b_s_d + b_s_c + b_s_v;

        let g_eff = if b_s_aero > 0.0 {
            (b_s_d * g_dust + b_s_c * g_cloud + b_s_v * g_volc) / b_s_aero
        } else {
            0.0
        };

        let phase_m = henyey_greenstein_phase_function(theta, g_eff);

        let sun_depth = sun_path_optical_depth(
            pos_i,
            s_dir,
            atmosphere,
            config.sun_samples,
        );

        if let Some(sun_tau) = sun_depth {
            let total_tau = beta_e_r0 * (tau_view_gas + sun_tau.gas_depth)
                + k_e_dust * (tau_view_dust + sun_tau.dust_depth)
                + k_e_cloud * (tau_view_cloud + sun_tau.cloud_depth)
                + k_e_volc * (tau_view_volc + sun_tau.volcanic_depth);

            let attenuation = if total_tau > 700.0 { 0.0 } else { (-total_tau).exp() };

            let scatter_coeff = beta_s_r0 * rho_g * phase_r + b_s_aero * phase_m;
            in_scatter_accum += solar_spectral_irradiance * attenuation * scatter_coeff * ds;
        }
    }

    let ray_total_tau = beta_e_r0 * tau_view_gas
        + k_e_dust * tau_view_dust
        + k_e_cloud * tau_view_cloud
        + k_e_volc * tau_view_volc;

    let transmittance = if ray_total_tau > 700.0 {
        0.0
    } else {
        (-ray_total_tau).exp().clamp(0.0, 1.0)
    };

    SphericalScatteringResult {
        in_scattered_radiance: in_scatter_accum.max(0.0),
        optical_depth: ray_total_tau.max(0.0),
        transmittance,
    }
}

pub fn spherical_sky_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    planet_radius: Length,
    surface_pressure: Pressure,
    surface_temperature: Temperature,
    gas_scale_height: Length,
    gas_props: &GasOpticalProperties,
    dust_profile: DustProfile,
    cloud_profile: CloudProfile,
    volcanic_profile: VolcanicProfile,
    config: &AtmosphericRaymarchConfig,
) -> SphericalScatteringResult {
    let atmosphere = SphericalAtmosphere::new(
        planet_radius,
        config.atmosphere_top_altitude,
        surface_pressure,
        surface_temperature,
        gas_scale_height,
        gas_props.clone(),
        dust_profile,
        cloud_profile,
        volcanic_profile,
    );

    single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_spectral_irradiance,
        wavelength,
        &atmosphere,
        config,
    )
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
            let result = single_scattering_spectral_radiance(
                ray_origin,
                ray_dir,
                sun_dir,
                solar_irradiance,
                wavelength,
                atmosphere,
                config,
            );

            let cmf = cie_color_matching_functions(wavelength);
            accumulated = accumulated + cmf * result.in_scattered_radiance;
        }

        lambda_m += step;
    }

    accumulated * step
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
    let linear_rgb = xyz_to_linear_srgb(xyz);
    let exposed = exposure_tone_map(linear_rgb, exposure);
    linear_to_srgb_gamma(exposed)
}

pub fn spherical_sky_rgb_fast(
    ray_origin: Vector3,
    ray_dir: Vector3,
    sun_dir: Vector3,
    solar_irradiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> (ColorRGB, ColorRGB) {
    let w_r = Wavelength::new(680.0e-9);
    let w_g = Wavelength::new(550.0e-9);
    let w_b = Wavelength::new(440.0e-9);

    let res_r = single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.r(),
        w_r,
        atmosphere,
        config,
    );

    let res_g = single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.g(),
        w_g,
        atmosphere,
        config,
    );

    let res_b = single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        sun_dir,
        solar_irradiance_rgb.b(),
        w_b,
        atmosphere,
        config,
    );

    let in_scattered = ColorRGB::new(
        res_r.in_scattered_radiance,
        res_g.in_scattered_radiance,
        res_b.in_scattered_radiance,
    );

    let transmittance = ColorRGB::new(
        res_r.transmittance,
        res_g.transmittance,
        res_b.transmittance,
    );

    (in_scattered, transmittance)
}

pub fn spherical_view_transmittance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> f64 {
    let v_dir = ray_dir.normalized();
    let segment = ray_atmosphere_segment(
        ray_origin,
        v_dir,
        atmosphere.planet_radius,
        atmosphere.atmosphere_top_radius,
    );

    let (t_start, t_end, _hits_ground) = match segment {
        Some(seg) => seg,
        None => {
            return 1.0;
        }
    };

    let total_path_len = t_end.value() - t_start.value();
    if total_path_len <= 0.0 || !total_path_len.is_finite() {
        return 1.0;
    }

    let depth = spherical_optical_depth_segment(
        ray_origin + v_dir * t_start.value(),
        ray_origin + v_dir * t_end.value(),
        atmosphere,
        config.view_samples,
    );

    let total_tau = depth.total_extinction_optical_depth(wavelength, atmosphere);
    if total_tau > 700.0 {
        0.0
    } else {
        (-total_tau).exp().clamp(0.0, 1.0)
    }
}

pub fn refracted_single_scattering_spectral_radiance(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
) -> SphericalScatteringResult {
    let up = ray_origin.normalized();
    let refr = atmosphere.gas_optical_properties.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(
        refr,
        atmosphere.surface_temperature,
        atmosphere.surface_pressure,
    );
    let s_refracted = refracted_sun_direction(
        geometric_sun_dir,
        up,
        refr_actual,
        atmosphere.gas_scale_height,
        atmosphere.planet_radius,
    );
    single_scattering_spectral_radiance(
        ray_origin,
        ray_dir,
        s_refracted,
        solar_spectral_irradiance,
        wavelength,
        atmosphere,
        config,
    )
}

pub fn stellar_disk_integrated_single_scattering(
    ray_origin: Vector3,
    ray_dir: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    solar_spectral_irradiance: f64,
    wavelength: Wavelength,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
    disk_samples: u32,
    limb_darkening_coeff: f64,
) -> SphericalScatteringResult {
    let samples = stellar_disk_sample_directions(
        geometric_sun_dir,
        star_angular_radius,
        disk_samples,
        limb_darkening_coeff,
    );

    let up = ray_origin.normalized();
    let refr = atmosphere.gas_optical_properties.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(
        refr,
        atmosphere.surface_temperature,
        atmosphere.surface_pressure,
    );

    let mut in_scatter_accum = 0.0;
    let mut opt_depth_accum = 0.0;
    let mut trans_accum = 0.0;

    for (s_dir, weight) in samples {
        let s_refracted = refracted_sun_direction(
            s_dir,
            up,
            refr_actual,
            atmosphere.gas_scale_height,
            atmosphere.planet_radius,
        );

        let res = single_scattering_spectral_radiance(
            ray_origin,
            ray_dir,
            s_refracted,
            solar_spectral_irradiance,
            wavelength,
            atmosphere,
            config,
        );

        in_scatter_accum += res.in_scattered_radiance * weight;
        opt_depth_accum += res.optical_depth * weight;
        trans_accum += res.transmittance * weight;
    }

    SphericalScatteringResult {
        in_scattered_radiance: in_scatter_accum,
        optical_depth: opt_depth_accum,
        transmittance: trans_accum,
    }
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
    let t_sun = solar_temperature.value();
    let theta_sun = star_angular_radius.value().clamp(0.0, PI / 2.0);

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
            let result = stellar_disk_integrated_single_scattering(
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
            );

            let cmf = cie_color_matching_functions(wavelength);
            accumulated = accumulated + cmf * result.in_scattered_radiance;
        }

        lambda_m += step;
    }

    accumulated * step
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
    let linear_rgb = xyz_to_linear_srgb(xyz);
    let exposed = exposure_tone_map(linear_rgb, exposure);
    linear_to_srgb_gamma(exposed)
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
    let w_r = Wavelength::new(680.0e-9);
    let w_g = Wavelength::new(550.0e-9);
    let w_b = Wavelength::new(440.0e-9);

    let res_r = stellar_disk_integrated_single_scattering(
        ray_origin,
        ray_dir,
        geometric_sun_dir,
        star_angular_radius,
        solar_irradiance_rgb.r(),
        w_r,
        atmosphere,
        config,
        disk_samples,
        limb_darkening_coeff,
    );

    let res_g = stellar_disk_integrated_single_scattering(
        ray_origin,
        ray_dir,
        geometric_sun_dir,
        star_angular_radius,
        solar_irradiance_rgb.g(),
        w_g,
        atmosphere,
        config,
        disk_samples,
        limb_darkening_coeff,
    );

    let res_b = stellar_disk_integrated_single_scattering(
        ray_origin,
        ray_dir,
        geometric_sun_dir,
        star_angular_radius,
        solar_irradiance_rgb.b(),
        w_b,
        atmosphere,
        config,
        disk_samples,
        limb_darkening_coeff,
    );

    let in_scattered = ColorRGB::new(
        res_r.in_scattered_radiance,
        res_g.in_scattered_radiance,
        res_b.in_scattered_radiance,
    );

    let transmittance = ColorRGB::new(
        res_r.transmittance,
        res_g.transmittance,
        res_b.transmittance,
    );

    (in_scattered, transmittance)
}

pub fn stellar_disk_direct_radiance_rgb(
    view_dir: Vector3,
    observer_pos: Vector3,
    geometric_sun_dir: Vector3,
    star_angular_radius: Angle,
    star_surface_radiance_rgb: ColorRGB,
    atmosphere: &SphericalAtmosphere,
    config: &AtmosphericRaymarchConfig,
    limb_darkening_coeff: f64,
) -> ColorRGB {
    let up = observer_pos.normalized();
    let refr = atmosphere.gas_optical_properties.refractivity_stp();
    let refr_actual = refractivity_at_temperature_pressure(
        refr,
        atmosphere.surface_temperature,
        atmosphere.surface_pressure,
    );

    let v_unrefracted = unrefracted_sun_direction(
        view_dir,
        up,
        refr_actual,
        atmosphere.gas_scale_height,
        atmosphere.planet_radius,
    );

    let s = geometric_sun_dir.normalized();
    let cos_angle = v_unrefracted.dot(&s).clamp(-1.0, 1.0);
    let angle_to_center = cos_angle.acos();
    let theta_max = star_angular_radius.value();

    if angle_to_center > theta_max || theta_max <= 0.0 {
        return ColorRGB::zero();
    }

    let rho = (angle_to_center / theta_max).clamp(0.0, 1.0);
    let mu = (1.0 - rho * rho).max(0.0).sqrt();
    let ld = stellar_limb_darkening(mu, limb_darkening_coeff);

    let w_r = Wavelength::new(680.0e-9);
    let w_g = Wavelength::new(550.0e-9);
    let w_b = Wavelength::new(440.0e-9);

    let trans_r = spherical_view_transmittance(observer_pos, view_dir, w_r, atmosphere, config);
    let trans_g = spherical_view_transmittance(observer_pos, view_dir, w_g, atmosphere, config);
    let trans_b = spherical_view_transmittance(observer_pos, view_dir, w_b, atmosphere, config);

    ColorRGB::new(
        star_surface_radiance_rgb.r() * ld * trans_r,
        star_surface_radiance_rgb.g() * ld * trans_g,
        star_surface_radiance_rgb.b() * ld * trans_b,
    )
}