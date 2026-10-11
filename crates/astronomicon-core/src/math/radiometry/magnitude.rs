use crate::math::radiometry::stellar_basics::orbital_irradiance;
use crate::units::constants::{
    IAU_2015_BOLOMETRIC_IRRADIANCE_ZERO_POINT, IAU_2015_BOLOMETRIC_LUMINOSITY_ZERO_POINT,
};
use crate::units::{Irradiance, Length, Luminosity};

pub fn absolute_bolometric_magnitude(luminosity: Luminosity) -> f64 {
    let l = luminosity.value();
    if !l.is_finite() || l <= 0.0 {
        return f64::INFINITY;
    }
    -2.5 * (l / IAU_2015_BOLOMETRIC_LUMINOSITY_ZERO_POINT).log10()
}

pub fn apparent_bolometric_magnitude(irradiance: Irradiance) -> f64 {
    let f = irradiance.value();
    if !f.is_finite() || f <= 0.0 {
        return f64::INFINITY;
    }
    -2.5 * (f / IAU_2015_BOLOMETRIC_IRRADIANCE_ZERO_POINT).log10()
}

pub fn apparent_bolometric_magnitude_from_luminosity(
    luminosity: Luminosity,
    distance: Length,
) -> f64 {
    let irr = orbital_irradiance(luminosity, distance);
    apparent_bolometric_magnitude(irr)
}

pub fn luminosity_from_absolute_bolometric_magnitude(magnitude: f64) -> Luminosity {
    if !magnitude.is_finite() {
        return Luminosity::new(0.0);
    }
    let exponent = -0.4 * magnitude;
    let l = IAU_2015_BOLOMETRIC_LUMINOSITY_ZERO_POINT * (10.0_f64).powf(exponent);
    if !l.is_finite() || l <= 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(l)
    }
}

pub fn irradiance_from_apparent_bolometric_magnitude(magnitude: f64) -> Irradiance {
    if !magnitude.is_finite() {
        return Irradiance::new(0.0);
    }
    let exponent = -0.4 * magnitude;
    let f = IAU_2015_BOLOMETRIC_IRRADIANCE_ZERO_POINT * (10.0_f64).powf(exponent);
    if !f.is_finite() || f <= 0.0 {
        Irradiance::new(0.0)
    } else {
        Irradiance::new(f)
    }
}
