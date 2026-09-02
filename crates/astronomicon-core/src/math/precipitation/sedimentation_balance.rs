use crate::math::aerosol::particle_terminal_velocity;
use crate::math::clouds::CloudMorphology;
use crate::units::{
    Acceleration, Density, DynamicViscosity, Length, SpecificEnergy, Speed,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SedimentationBalanceResult {
    pub critical_radius: Length,
    pub mean_droplet_radius: Length,
    pub vertical_velocity_scale: Speed,
    pub sedimentable_fraction: f64,
}

pub fn convective_velocity_scale(cape: SpecificEnergy) -> Speed {
    let c = cape.value();
    if !c.is_finite() || c <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new((2.0 * c).sqrt())
    }
}

pub fn turbulent_velocity_scale(
    layer_thickness: Length,
    vertical_wind_shear: f64,
) -> Speed {
    let dz = layer_thickness.value();
    let shear = vertical_wind_shear.abs();
    if !dz.is_finite() || !shear.is_finite() || dz <= 0.0 || shear <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(dz * shear)
    }
}

pub fn layer_vertical_velocity_scale(
    morphology: CloudMorphology,
    cape: SpecificEnergy,
    layer_thickness: Length,
    vertical_wind_shear: f64,
) -> Speed {
    match morphology {
        CloudMorphology::Convective => convective_velocity_scale(cape),
        CloudMorphology::Stratiform => {
            turbulent_velocity_scale(layer_thickness, vertical_wind_shear)
        }
    }
}

pub fn critical_sedimentation_radius(
    gravity: Acceleration,
    particle_density: Density,
    fluid_density: Density,
    dynamic_viscosity: DynamicViscosity,
    vertical_velocity: Speed,
) -> Length {
    let w = vertical_velocity.value();
    if !w.is_finite() || w <= 0.0 {
        return Length::new(0.0);
    }

    let mut low = 1.0e-7;
    let mut high = 0.05;

    while particle_terminal_velocity(
        gravity,
        particle_density,
        fluid_density,
        Length::new(high),
        dynamic_viscosity,
    )
    .value() < w
        && high < 1.0
    {
        high *= 2.0;
    }

    for _ in 0..50 {
        let mid = 0.5 * (low + high);
        let v = particle_terminal_velocity(
            gravity,
            particle_density,
            fluid_density,
            Length::new(mid),
            dynamic_viscosity,
        )
        .value();

        if v < w {
            low = mid;
        } else {
            high = mid;
        }
    }

    Length::new(0.5 * (low + high))
}

pub fn droplet_volume_mean_radius(
    condensate_density: Density,
    particle_density: Density,
    ccn_factor: Option<f64>,
) -> Length {
    let rho_c = condensate_density.value();
    let rho_p = particle_density.value();

    if !rho_c.is_finite() || !rho_p.is_finite() || rho_c <= 0.0 || rho_p <= 0.0 {
        return Length::new(0.0);
    }

    let f_ccn = ccn_factor.unwrap_or(1.0).clamp(0.01, 100.0);
    let n_base = 1.0e8;
    let n_droplets = n_base * f_ccn;

    let total_volume = rho_c / rho_p;
    let mean_volume = total_volume / n_droplets;
    let r3 = (3.0 * mean_volume) / (4.0 * std::f64::consts::PI);

    if !r3.is_finite() || r3 <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(r3.cbrt())
    }
}

pub fn sedimentable_mass_fraction(
    critical_radius: Length,
    mean_radius: Length,
) -> f64 {
    let r_crit = critical_radius.value();
    let r_0 = mean_radius.value();

    if !r_0.is_finite() || r_0 <= 0.0 {
        return 0.0;
    }

    if !r_crit.is_finite() || r_crit <= 0.0 {
        return 1.0;
    }

    let u = r_crit / r_0;
    if u >= 50.0 {
        return 0.0;
    }

    let poly = 1.0 + u + 0.5 * u * u + (1.0 / 6.0) * u * u * u;
    let fraction = (-u).exp() * poly;

    fraction.clamp(0.0, 1.0)
}

pub fn resolve_sedimentation_balance(
    condensate_density: Density,
    particle_density: Density,
    fluid_density: Density,
    dynamic_viscosity: DynamicViscosity,
    gravity: Acceleration,
    vertical_velocity: Speed,
    ccn_factor: Option<f64>,
) -> SedimentationBalanceResult {
    let r_mean = droplet_volume_mean_radius(condensate_density, particle_density, ccn_factor);
    let r_crit = critical_sedimentation_radius(
        gravity,
        particle_density,
        fluid_density,
        dynamic_viscosity,
        vertical_velocity,
    );
    let fraction = sedimentable_mass_fraction(r_crit, r_mean);

    SedimentationBalanceResult {
        critical_radius: r_crit,
        mean_droplet_radius: r_mean,
        vertical_velocity_scale: vertical_velocity,
        sedimentable_fraction: fraction,
    }
}