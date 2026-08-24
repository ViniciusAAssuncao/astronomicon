
use crate::units::{Acceleration, MassAttenuationCoefficient, Pressure};

pub fn atmospheric_mass_column(surface_pressure: Pressure, gravity: Acceleration) -> f64 {
    let p = surface_pressure.value();
    let g = gravity.value();

    if p <= 0.0 || g <= 0.0 || !p.is_finite() || !g.is_finite() {
        return 0.0;
    }

    p / g
}

pub fn atmospheric_transmission(
    mass_column: f64,
    mean_attenuation_coeff: MassAttenuationCoefficient,
) -> f64 {
    let x = mass_column;
    let mu = mean_attenuation_coeff.value();

    if x <= 0.0 || mu <= 0.0 || !x.is_finite() || !mu.is_finite() {
        return 1.0;
    }

    let optical_depth = mu * x;
    if optical_depth < 0.0 {
        return 1.0;
    }

    (-optical_depth).exp().clamp(0.0, 1.0)
}
