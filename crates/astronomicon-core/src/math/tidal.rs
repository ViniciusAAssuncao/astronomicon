use crate::units::constants::ROCHE_FLUID_COEFFICIENT;
use crate::units::{Density, Duration, GravitationalParameter, Length};
use std::f64::consts::PI;

pub fn roche_limit_rigid(
    primary_radius: Length,
    primary_density: Density,
    satellite_density: Density,
) -> Length {
    let r_p = primary_radius.value();
    let rho_p = primary_density.value();
    let rho_s = satellite_density.value();

    if r_p <= 0.0
        || rho_p <= 0.0
        || rho_s <= 0.0
        || !r_p.is_finite()
        || !rho_p.is_finite()
        || !rho_s.is_finite()
    {
        return Length::new(0.0);
    }

    let ratio = 2.0 * rho_p / rho_s;
    Length::new(r_p * ratio.cbrt())
}

pub fn roche_limit_fluid(
    primary_radius: Length,
    primary_density: Density,
    satellite_density: Density,
) -> Length {
    let r_p = primary_radius.value();
    let rho_p = primary_density.value();
    let rho_s = satellite_density.value();

    if r_p <= 0.0
        || rho_p <= 0.0
        || rho_s <= 0.0
        || !r_p.is_finite()
        || !rho_p.is_finite()
        || !rho_s.is_finite()
    {
        return Length::new(0.0);
    }

    let ratio = rho_p / rho_s;
    Length::new(ROCHE_FLUID_COEFFICIENT * r_p * ratio.cbrt())
}

pub fn synchronous_orbit_radius(
    mu_primary: GravitationalParameter,
    rotation_period: Duration,
) -> Length {
    let mu = mu_primary.value();
    let t = rotation_period.value();

    if mu <= 0.0 || t <= 0.0 || !mu.is_finite() || !t.is_finite() {
        return Length::new(0.0);
    }

    let val = (mu * t * t) / (4.0 * PI * PI);
    Length::new(val.cbrt())
}