use crate::math::black_hole::horizon_geometry::gravitational_radius;
use crate::units::constants::{
    BOLTZMANN_CONSTANT, GRAVITATIONAL_CONSTANT, PLANCK_CONSTANT, PROTON_MASS, SPEED_OF_LIGHT,
    STEFAN_BOLTZMANN_CONSTANT, THOMSON_CROSS_SECTION, THORNE_SPIN_LIMIT,
};
use crate::units::{Luminosity, Mass, Temperature};
use std::f64::consts::PI;

pub fn eddington_luminosity(mass: Mass) -> Luminosity {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Luminosity::new(0.0);
    }
    let l_edd = (4.0 * PI * GRAVITATIONAL_CONSTANT * SPEED_OF_LIGHT * PROTON_MASS * m)
        / THOMSON_CROSS_SECTION;
    if !l_edd.is_finite() || l_edd < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(l_edd)
    }
}

pub fn hawking_temperature(mass: Mass, dimensionless_spin: f64) -> Temperature {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Temperature::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let sqrt_term = (1.0 - a_star * a_star).max(0.0).sqrt();
    let num = PLANCK_CONSTANT * SPEED_OF_LIGHT.powi(3) * sqrt_term;
    let den = 8.0 * PI * PI * BOLTZMANN_CONSTANT * GRAVITATIONAL_CONSTANT * m * (1.0 + sqrt_term);
    if den <= 0.0 || !den.is_finite() {
        return Temperature::new(0.0);
    }
    let t_h = num / den;
    if !t_h.is_finite() || t_h < 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(t_h)
    }
}

pub fn hawking_luminosity(mass: Mass, dimensionless_spin: f64) -> Luminosity {
    let m = mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Luminosity::new(0.0);
    }
    let a_star = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let rg = gravitational_radius(mass).value();
    let sqrt_term = (1.0 - a_star * a_star).max(0.0).sqrt();
    let area = 8.0 * PI * rg * rg * (1.0 + sqrt_term);
    let t_h = hawking_temperature(mass, dimensionless_spin).value();
    let l_h = STEFAN_BOLTZMANN_CONSTANT * area * t_h.powi(4);
    if !l_h.is_finite() || l_h < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(l_h)
    }
}

pub fn radiative_efficiency(dimensionless_spin: f64) -> f64 {
    let a = dimensionless_spin.clamp(0.0, THORNE_SPIN_LIMIT);
    let z1 = 1.0 + (1.0 - a * a).cbrt() * ((1.0 + a).cbrt() + (1.0 - a).cbrt());
    let z2 = (3.0 * a * a + z1 * z1).sqrt();
    let term = ((3.0 - z1) * (3.0 + z1 + 2.0 * z2)).max(0.0).sqrt();
    let r = 3.0 + z2 - term;
    if r <= 0.0 || !r.is_finite() {
        return 0.0572;
    }
    let r_sqrt = r.sqrt();
    let num = r * r_sqrt - 2.0 * r_sqrt + a;
    let den_inside = r * r_sqrt - 3.0 * r_sqrt + 2.0 * a;
    if den_inside <= 0.0 {
        return 1.0 - 1.0 / 3.0_f64.sqrt();
    }
    let den = r.powf(0.75) * den_inside.sqrt();
    if den <= 0.0 || !den.is_finite() {
        return 0.0572;
    }
    let energy = num / den;
    (1.0 - energy).clamp(0.0, 1.0)
}
