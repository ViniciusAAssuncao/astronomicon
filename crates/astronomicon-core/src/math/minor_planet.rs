use crate::domain::SpectralType;
use crate::units::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{Density, Duration, Length};
use std::f64::consts::PI;

pub fn triaxial_ellipsoid_volume(axis_a: Length, axis_b: Length, axis_c: Length) -> f64 {
    let a = axis_a.value();
    let b = axis_b.value();
    let c = axis_c.value();

    if a <= 0.0 || b <= 0.0 || c <= 0.0 || !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return 0.0;
    }

    (4.0 / 3.0) * PI * a * b * c
}

pub fn triaxial_ellipsoid_surface_area(axis_a: Length, axis_b: Length, axis_c: Length) -> f64 {
    let a = axis_a.value();
    let b = axis_b.value();
    let c = axis_c.value();

    if a <= 0.0 || b <= 0.0 || c <= 0.0 || !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return 0.0;
    }

    let p = 1.6075;
    let ab_p = (a * b).powf(p);
    let ac_p = (a * c).powf(p);
    let bc_p = (b * c).powf(p);

    let sum = (ab_p + ac_p + bc_p) / 3.0;
    4.0 * PI * sum.powf(1.0 / p)
}

pub fn equivalent_spherical_radius(axis_a: Length, axis_b: Length, axis_c: Length) -> Length {
    let a = axis_a.value();
    let b = axis_b.value();
    let c = axis_c.value();

    if a <= 0.0 || b <= 0.0 || c <= 0.0 || !a.is_finite() || !b.is_finite() || !c.is_finite() {
        return Length::new(0.0);
    }

    Length::new((a * b * c).cbrt())
}

pub fn grain_density_by_spectral_type(spectral_type: SpectralType) -> Density {
    match spectral_type {
        SpectralType::C => Density::new(1300.0),
        SpectralType::S => Density::new(2700.0),
        SpectralType::M => Density::new(5300.0),
        SpectralType::D => Density::new(1500.0),
        SpectralType::V => Density::new(3000.0),
        SpectralType::P => Density::new(1400.0),
    }
}

pub fn bulk_density(grain_density: Density, macroporosity: f64) -> Density {
    let rho_g = grain_density.value();
    if rho_g <= 0.0 || !rho_g.is_finite() {
        return Density::new(0.0);
    }

    let phi = if macroporosity.is_finite() {
        macroporosity.clamp(0.0, 1.0)
    } else {
        0.0
    };

    Density::new(rho_g * (1.0 - phi))
}

pub fn critical_rotation_period(bulk_density: Density) -> Duration {
    let rho = bulk_density.value();
    if rho <= 0.0 || !rho.is_finite() {
        return Duration::new(0.0);
    }

    let period = (3.0 * PI / (GRAVITATIONAL_CONSTANT * rho)).sqrt();
    Duration::new(period)
}
