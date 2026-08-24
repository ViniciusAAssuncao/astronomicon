use crate::chemistry::optics::GasOpticalProperties;
use crate::math::optics::mass_optical_efficiencies;
use crate::units::constants::OPTICAL_REFERENCE_WAVELENGTH;
use crate::units::{Density, Length, Pressure, Temperature, Wavelength};
use serde::{Deserialize, Serialize};

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
        let (ke, ks, _, g, alpha) = mass_optical_efficiencies(
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
        if z < 0.0
            || rho0 <= 0.0
            || h <= 0.0
            || !z.is_finite()
            || !rho0.is_finite()
            || !h.is_finite()
        {
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
        let (ke, ks, _, g, alpha) = mass_optical_efficiencies(
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

        if z < z_lcl
            || z > z_top
            || z_top <= z_lcl
            || cov <= 0.0
            || rho_base <= 0.0
            || !z.is_finite()
        {
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
        let (ke, ks, _, g, alpha) = mass_optical_efficiencies(
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

        if rho_peak <= 0.0
            || h_plume <= 0.0
            || !z.is_finite()
            || !rho_peak.is_finite()
            || !h_plume.is_finite()
        {
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
        let top_r =
            Length::new(planet_radius.value() + atmosphere_top_altitude.value().max(1000.0));
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
