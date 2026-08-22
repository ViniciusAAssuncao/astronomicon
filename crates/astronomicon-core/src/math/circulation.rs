use crate::units::{Acceleration, Length, Speed};
use std::f64::consts::PI;

pub fn equatorial_rossby_deformation_radius(
    gravity: Acceleration,
    scale_height: Length,
    beta: f64,
) -> Length {
    let g = gravity.value();
    let h = scale_height.value();

    if g <= 0.0 || h <= 0.0 || beta <= 0.0 || !g.is_finite() || !h.is_finite() || !beta.is_finite()
    {
        return Length::new(0.0);
    }

    let c = (g * h).sqrt();
    let lr = (c / beta).sqrt();

    Length::new(lr)
}

pub fn rhines_scale(characteristic_velocity: Speed, beta: f64) -> Length {
    let u = characteristic_velocity.value();

    if u <= 0.0 || beta <= 0.0 || !u.is_finite() || !beta.is_finite() {
        return Length::new(0.0);
    }

    let l_beta = (u / beta).sqrt();
    Length::new(l_beta)
}

pub fn circulation_cells_per_hemisphere(planet_radius: Length, rhines_scale: Length) -> u32 {
    let r = planet_radius.value();
    let l_beta = rhines_scale.value();

    if r <= 0.0 || l_beta <= 0.0 || !r.is_finite() || !l_beta.is_finite() {
        return 1;
    }

    let quarter_meridian = r * (PI / 2.0);
    let ratio = quarter_meridian / l_beta;

    if ratio < 1.0 || !ratio.is_finite() {
        1
    } else {
        ratio.floor() as u32
    }
}
