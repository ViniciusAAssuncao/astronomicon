use crate::chemistry::optics::{
    mean_refractivity_gladstone_dale,
    mean_refractivity_lorentz_lorenz,
};
use crate::error::DomainResult;
use crate::math::optics::mass_optical_efficiencies;
use crate::math::thermodynamics::MatterState;
use crate::math::volcanism::VolcanicEruptionStyle;
use crate::units::constants::{ OPTICAL_REFERENCE_WAVELENGTH, STANDARD_ATMOSPHERE_PRESSURE, STANDARD_GRAVITY, STP_TEMPERATURE };
use crate::units::{
    Acceleration,
    Density,
    Duration,
    DynamicViscosity,
    Length,
    MassRate,
    Pressure,
    Speed,
    Temperature,
    Wavelength,
};
use serde::{ Deserialize, Serialize };
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericAerosolProperties {
    dust_density: Density,
    volcanic_density: Density,
    cloud_density: Density,
    total_density: Density,
    asymmetry_factor_g: f64,
    base_extinction_coefficient: f64,
    base_scattering_coefficient: f64,
    angstrom_exponent: f64,
}

impl AtmosphericAerosolProperties {
    pub fn new(
        dust_density: Density,
        volcanic_density: Density,
        cloud_density: Density,
        total_density: Density,
        asymmetry_factor_g: f64,
        base_extinction_coefficient: f64,
        base_scattering_coefficient: f64,
        angstrom_exponent: f64,
    ) -> Self {
        Self {
            dust_density,
            volcanic_density,
            cloud_density,
            total_density,
            asymmetry_factor_g,
            base_extinction_coefficient,
            base_scattering_coefficient,
            angstrom_exponent,
        }
    }

    pub fn dust_density(&self) -> Density {
        self.dust_density
    }

    pub fn volcanic_density(&self) -> Density {
        self.volcanic_density
    }

    pub fn cloud_density(&self) -> Density {
        self.cloud_density
    }

    pub fn total_density(&self) -> Density {
        self.total_density
    }

    pub fn asymmetry_factor_g(&self) -> f64 {
        self.asymmetry_factor_g
    }

    pub fn base_extinction_coefficient(&self) -> f64 {
        self.base_extinction_coefficient
    }

    pub fn base_scattering_coefficient(&self) -> f64 {
        self.base_scattering_coefficient
    }

    pub fn angstrom_exponent(&self) -> f64 {
        self.angstrom_exponent
    }
}

pub fn particle_terminal_velocity(
    gravity: Acceleration,
    particle_density: Density,
    fluid_density: Density,
    particle_radius: Length,
    dynamic_viscosity: DynamicViscosity,
) -> Speed {
    let g = gravity.value();
    let rho_p = particle_density.value();
    let rho_f = fluid_density.value();
    let r = particle_radius.value();
    let eta = dynamic_viscosity.value();

    if g <= 0.0
        || rho_p <= rho_f
        || rho_p <= 0.0
        || r <= 0.0
        || eta <= 0.0
        || !g.is_finite()
        || !rho_p.is_finite()
        || !rho_f.is_finite()
        || !r.is_finite()
        || !eta.is_finite()
    {
        return Speed::new(0.0);
    }

    let delta_rho = rho_p - rho_f;
    let v_term = (2.0 / 9.0) * (delta_rho * g * r * r) / eta;

    if !v_term.is_finite() || v_term <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(v_term)
    }
}

pub fn dynamic_aerosol_scale_height(
    gas_scale_height: Length,
    surface_gravity: Acceleration,
    eddy_diffusion_coefficient: f64,
    particle_density: Density,
    fluid_density: Density,
    particle_radius: Length,
    dynamic_viscosity: DynamicViscosity,
) -> Length {
    let h_gas = gas_scale_height.value();
    let k_zz = eddy_diffusion_coefficient;

    if h_gas <= 0.0 || k_zz <= 0.0 || !h_gas.is_finite() || !k_zz.is_finite() {
        return Length::new(0.0);
    }

    let v_term = particle_terminal_velocity(
        surface_gravity,
        particle_density,
        fluid_density,
        particle_radius,
        dynamic_viscosity,
    )
    .value();

    let denom = k_zz + v_term * h_gas;
    if denom <= 0.0 || !denom.is_finite() {
        return gas_scale_height;
    }

    let h_aero = (h_gas * k_zz) / denom;
    if !h_aero.is_finite() || h_aero <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(h_aero.min(h_gas))
    }
}

pub fn derived_aerosol_scale_height(
    surface_gravity: Acceleration,
    gas_scale_height: Length,
    atmospheric_density: Density,
) -> Length {
    let default_particle_density = Density::new(2500.0);
    let default_particle_radius = Length::new(1.5e-6);
    let default_viscosity = DynamicViscosity::new(1.81e-5);
    let default_k_zz = 1.2;

    dynamic_aerosol_scale_height(
        gas_scale_height,
        surface_gravity,
        default_k_zz,
        default_particle_density,
        atmospheric_density,
        default_particle_radius,
        default_viscosity,
    )
}

pub fn aerosol_density_at_altitude(
    surface_density: Density,
    altitude: Length,
    aerosol_scale_height: Length,
) -> Density {
    let rho_0 = surface_density.value();
    let z = altitude.value();
    let h_aero = aerosol_scale_height.value();

    if rho_0 <= 0.0 || !rho_0.is_finite() {
        return Density::new(0.0);
    }

    if z <= 0.0 {
        return surface_density;
    }

    if h_aero <= 0.0 || !h_aero.is_finite() || !z.is_finite() {
        return Density::new(0.0);
    }

    let exponent = -z / h_aero;
    if exponent < -700.0 {
        return Density::new(0.0);
    }

    let rho = rho_0 * exponent.exp();
    if !rho.is_finite() || rho <= 0.0 {
        Density::new(0.0)
    } else {
        Density::new(rho)
    }
}

pub fn aerosol_properties_at_altitude(
    surface_properties: &AtmosphericAerosolProperties,
    altitude: Length,
    aerosol_scale_height: Length,
) -> AtmosphericAerosolProperties {
    let z = altitude.value();
    let h_aero = aerosol_scale_height.value();

    if z <= 0.0 {
        return *surface_properties;
    }

    if h_aero <= 0.0 || !h_aero.is_finite() || !z.is_finite() {
        return AtmosphericAerosolProperties::new(
            Density::new(0.0),
            Density::new(0.0),
            Density::new(0.0),
            Density::new(0.0),
            surface_properties.asymmetry_factor_g(),
            0.0,
            0.0,
            surface_properties.angstrom_exponent(),
        );
    }

    let exponent = -z / h_aero;
    let factor = if exponent < -700.0 {
        0.0
    } else {
        exponent.exp()
    };

    AtmosphericAerosolProperties::new(
        Density::new(surface_properties.dust_density().value() * factor),
        Density::new(surface_properties.volcanic_density().value() * factor),
        Density::new(surface_properties.cloud_density().value() * factor),
        Density::new(surface_properties.total_density().value() * factor),
        surface_properties.asymmetry_factor_g(),
        surface_properties.base_extinction_coefficient() * factor,
        surface_properties.base_scattering_coefficient() * factor,
        surface_properties.angstrom_exponent(),
    )
}

pub fn dust_threshold_friction_velocity(
    gravity: Acceleration,
    atmospheric_density: Density,
    grain_density: Density,
    grain_diameter: Length,
) -> Speed {
    let g = gravity.value();
    let rho_a = atmospheric_density.value();
    let rho_p = grain_density.value();
    let d = grain_diameter.value();

    if g <= 0.0
        || rho_a <= 0.0
        || rho_p <= rho_a
        || d <= 0.0
        || !g.is_finite()
        || !rho_a.is_finite()
        || !rho_p.is_finite()
        || !d.is_finite()
    {
        return Speed::new(0.0);
    }

    let a_coeff = 0.118;
    let b_cohesion = 1.6e-4;
    let cohesion_term = 1.0 + b_cohesion / (rho_a * g * d);
    let density_ratio = (rho_p - rho_a) / rho_a;
    let u_star_t = a_coeff * (density_ratio * g * d * cohesion_term).sqrt();

    if !u_star_t.is_finite() || u_star_t <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(u_star_t)
    }
}

pub fn dust_threshold_surface_wind(gravity: Acceleration, atmospheric_density: Density) -> Speed {
    let grain_density = Density::new(2650.0);
    let grain_diameter = Length::new(1.0e-4);
    let u_star_t = dust_threshold_friction_velocity(
        gravity,
        atmospheric_density,
        grain_density,
        grain_diameter,
    );
    let drag_coeff: f64 = 0.003;
    let ratio = 1.0 / drag_coeff.sqrt();
    Speed::new(u_star_t.value() * ratio)
}

pub fn airborne_dust_density(
    surface_wind_speed: Speed,
    threshold_wind_speed: Speed,
    atmospheric_density: Density,
    surface_gravity: Acceleration,
    dust_availability_factor: f64,
    surface_coverage_fraction: f64,
    surface_humidity: f64,
) -> Density {
    let v = surface_wind_speed.value();
    let v_t = threshold_wind_speed.value();
    let rho_a = atmospheric_density.value();
    let g = surface_gravity.value();

    if v <= v_t
        || v_t <= 0.0
        || rho_a <= 0.0
        || g <= 0.0
        || !v.is_finite()
        || !v_t.is_finite()
        || !rho_a.is_finite()
        || !g.is_finite()
    {
        return Density::new(0.0);
    }

    let f_dust = dust_availability_factor.clamp(0.0, 1.0);
    let f_land = (1.0 - surface_coverage_fraction.clamp(0.0, 1.0)).max(0.0);
    let h = surface_humidity.clamp(0.0, 1.0);
    let f_moisture = (1.0 - h).powi(2);
    let suppression = f_dust * f_land * f_moisture;

    if suppression <= 0.0 || !suppression.is_finite() {
        return Density::new(0.0);
    }

    let delta_v = v - v_t;
    let normalized_excess = delta_v / (v_t + 0.1);
    let g_scale = (STANDARD_GRAVITY / g).clamp(0.1, 10.0);
    let c_dust = 1.5e-5;

    let rho_dust = c_dust * rho_a * normalized_excess * normalized_excess * g_scale * suppression;
    let clamped_dust = rho_dust.clamp(0.0, 0.05);

    if !clamped_dust.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(clamped_dust)
    }
}

pub fn volcanic_sulfate_aerosol_density(
    so2_mass_rate: MassRate,
    planet_radius: Length,
    scale_height: Length,
    residence_time: Duration,
) -> Density {
    let q_so2 = so2_mass_rate.value();
    let r = planet_radius.value();
    let h = scale_height.value();
    let tau = residence_time.value();

    if q_so2 <= 0.0
        || r <= 0.0
        || h <= 0.0
        || tau <= 0.0
        || !q_so2.is_finite()
        || !r.is_finite()
        || !h.is_finite()
        || !tau.is_finite()
    {
        return Density::new(0.0);
    }

    let volume_atm = 4.0 * PI * r * r * h;
    if volume_atm <= 0.0 || !volume_atm.is_finite() {
        return Density::new(0.0);
    }

    let conversion_factor = 1.53;
    let sulfate_mass = q_so2 * conversion_factor * tau;
    let rho = (sulfate_mass / volume_atm).clamp(0.0, 0.01);

    if !rho.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(rho)
    }
}

pub fn volcanic_ash_aerosol_density(
    eruption_style: VolcanicEruptionStyle,
    magma_extrusion_rate: MassRate,
    planet_radius: Length,
    scale_height: Length,
    residence_time: Duration,
) -> Density {
    let q_magma = magma_extrusion_rate.value();
    let r = planet_radius.value();
    let h = scale_height.value();
    let tau = residence_time.value();

    if q_magma <= 0.0
        || r <= 0.0
        || h <= 0.0
        || tau <= 0.0
        || !q_magma.is_finite()
        || !r.is_finite()
        || !h.is_finite()
        || !tau.is_finite()
    {
        return Density::new(0.0);
    }

    let ash_fraction = match eruption_style {
        VolcanicEruptionStyle::Explosive => 0.15,
        VolcanicEruptionStyle::Effusive => 0.001,
        VolcanicEruptionStyle::Cryovolcanic => 0.02,
        VolcanicEruptionStyle::SubaqueousEffusive | VolcanicEruptionStyle::Inactive => 0.0,
    };

    if ash_fraction <= 0.0 {
        return Density::new(0.0);
    }

    let volume_atm = 4.0 * PI * r * r * h;
    if volume_atm <= 0.0 || !volume_atm.is_finite() {
        return Density::new(0.0);
    }

    let ash_mass = q_magma * ash_fraction * tau;
    let rho = (ash_mass / volume_atm).clamp(0.0, 0.05);

    if !rho.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(rho)
    }
}

pub fn volcanic_aerosol_density(
    so2_mass_rate: MassRate,
    eruption_style: VolcanicEruptionStyle,
    magma_extrusion_rate: MassRate,
    planet_radius: Length,
    scale_height: Length,
) -> Density {
    let tau_sulfate = Duration::new(2.592e6);
    let tau_ash = Duration::new(6.048e5);

    let rho_sulfate = volcanic_sulfate_aerosol_density(
        so2_mass_rate,
        planet_radius,
        scale_height,
        tau_sulfate,
    );
    let rho_ash = volcanic_ash_aerosol_density(
        eruption_style,
        magma_extrusion_rate,
        planet_radius,
        scale_height,
        tau_ash,
    );

    Density::new(rho_sulfate.value() + rho_ash.value())
}

pub fn cloud_condensate_density(
    matter_state: MatterState,
    ocean_coverage_fraction: f64,
    surface_temperature: Temperature,
    surface_pressure: Pressure,
) -> Density {
    let cov = ocean_coverage_fraction.clamp(0.0, 1.0);
    let t = surface_temperature.value();
    let p = surface_pressure.value();

    if cov <= 0.0 || t <= 0.0 || p <= 0.0 || !cov.is_finite() || !t.is_finite() || !p.is_finite() {
        return Density::new(0.0);
    }

    let base_droplet_density = 2.0e-9;
    let pressure_factor = (p / STANDARD_ATMOSPHERE_PRESSURE).clamp(0.01, 100.0).sqrt();

    let rho = match matter_state {
        MatterState::Liquid => {
            let temp_factor = (t / 288.15).clamp(0.1, 2.0);
            cov * base_droplet_density * temp_factor * pressure_factor
        }
        MatterState::Solid => {
            let temp_factor = (t / 273.15).clamp(0.05, 1.0).powi(2);
            cov * 0.5 * base_droplet_density * temp_factor * pressure_factor
        }
        MatterState::Supercritical => 2.0e-8 * pressure_factor.min(5.0),
        MatterState::Vapor => 0.0,
    };

    let clamped = rho.clamp(0.0, 0.01);
    if !clamped.is_finite() {
        Density::new(0.0)
    } else {
        Density::new(clamped)
    }
}

pub fn composite_aerosol_properties_from_materials(
    dust_density: Density,
    volcanic_density: Density,
    cloud_density: Density,
    dust_refractive_index: (f64, f64),
    dust_particle_density: Density,
    volcanic_refractive_index: (f64, f64),
    volcanic_particle_density: Density,
    cloud_refractive_index: (f64, f64),
    cloud_particle_density: Density,
    reference_wavelength: Wavelength,
) -> AtmosphericAerosolProperties {
    let d = dust_density.value().max(0.0);
    let v = volcanic_density.value().max(0.0);
    let c = cloud_density.value().max(0.0);
    let total = d + v + c;

    if total <= 0.0 || !total.is_finite() {
        return AtmosphericAerosolProperties::new(
            dust_density,
            volcanic_density,
            cloud_density,
            Density::new(0.0),
            0.0,
            0.0,
            0.0,
            1.0,
        );
    }

    let r_dust = Length::new(1.0e-6);
    let r_volc = Length::new(5.0e-6);
    let r_cloud = Length::new(10.0e-6);

    let (ke_dust, ks_dust, _, g_dust, alpha_dust) = mass_optical_efficiencies(
        r_dust,
        dust_particle_density,
        dust_refractive_index.0,
        dust_refractive_index.1,
        reference_wavelength,
    );

    let (ke_volc, ks_volc, _, g_volc, alpha_volc) = mass_optical_efficiencies(
        r_volc,
        volcanic_particle_density,
        volcanic_refractive_index.0,
        volcanic_refractive_index.1,
        reference_wavelength,
    );

    let (ke_cloud, ks_cloud, _, g_cloud, alpha_cloud) = mass_optical_efficiencies(
        r_cloud,
        cloud_particle_density,
        cloud_refractive_index.0,
        cloud_refractive_index.1,
        reference_wavelength,
    );

    let scatt_dust = d * ks_dust;
    let scatt_volc = v * ks_volc;
    let scatt_cloud = c * ks_cloud;
    let total_scatt = scatt_dust + scatt_volc + scatt_cloud;

    let ext_dust = d * ke_dust;
    let ext_volc = v * ke_volc;
    let ext_cloud = c * ke_cloud;
    let total_ext = ext_dust + ext_volc + ext_cloud;

    let g_weighted = if total_scatt > 0.0 {
        (scatt_dust * g_dust + scatt_volc * g_volc + scatt_cloud * g_cloud) / total_scatt
    } else {
        0.0
    };

    let alpha_weighted = if total_scatt > 0.0 {
        (scatt_dust * alpha_dust + scatt_volc * alpha_volc + scatt_cloud * alpha_cloud)
            / total_scatt
    } else {
        1.0
    };

    AtmosphericAerosolProperties::new(
        dust_density,
        volcanic_density,
        cloud_density,
        Density::new(total),
        g_weighted.clamp(-0.99, 0.99),
        total_ext,
        total_scatt,
        alpha_weighted.clamp(0.0, 4.0),
    )
}

pub fn composite_aerosol_properties(
    dust_density: Density,
    volcanic_density: Density,
    cloud_density: Density,
) -> AtmosphericAerosolProperties {
    composite_aerosol_properties_from_materials(
        dust_density,
        volcanic_density,
        cloud_density,
        (1.55, 0.005),
        Density::new(2650.0),
        (1.52, 0.015),
        Density::new(2400.0),
        (1.333, 1.0e-8),
        Density::new(1000.0),
        Wavelength::new(OPTICAL_REFERENCE_WAVELENGTH),
    )
}

pub fn mie_scattering_coefficient_at_wavelength(
    base_scattering_coeff: f64,
    wavelength: Wavelength,
    reference_wavelength: Wavelength,
    angstrom_exponent: f64,
) -> f64 {
    let beta_0 = base_scattering_coeff;
    let lambda = wavelength.value();
    let lambda_0 = reference_wavelength.value();
    let alpha = angstrom_exponent;

    if beta_0 <= 0.0
        || lambda <= 0.0
        || lambda_0 <= 0.0
        || !beta_0.is_finite()
        || !lambda.is_finite()
        || !lambda_0.is_finite()
        || !alpha.is_finite()
    {
        return 0.0;
    }

    let ratio = lambda_0 / lambda;
    let beta = beta_0 * ratio.powf(alpha);

    if !beta.is_finite() || beta <= 0.0 {
        0.0
    } else {
        beta
    }
}

pub fn atmospheric_refractivity_lorentz_lorenz(composition: &[(String, f64)]) -> DomainResult<f64> {
    mean_refractivity_lorentz_lorenz(composition)
}

pub fn atmospheric_refractivity_gladstone_dale(composition: &[(String, f64)]) -> DomainResult<f64> {
    mean_refractivity_gladstone_dale(composition)
}

pub fn refractivity_at_temperature_pressure(
    refractivity_stp: f64,
    temperature: Temperature,
    pressure: Pressure,
) -> f64 {
    let t = temperature.value();
    let p = pressure.value();

    if t <= 0.0 || p <= 0.0 || !t.is_finite() || !p.is_finite() || !refractivity_stp.is_finite() {
        return 0.0;
    }

    let density_ratio = (p / STANDARD_ATMOSPHERE_PRESSURE) * (STP_TEMPERATURE / t);
    refractivity_stp * density_ratio
}