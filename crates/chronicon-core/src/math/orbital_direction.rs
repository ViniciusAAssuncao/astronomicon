use astronomicon_core::units::Angle;
use std::f64::consts::FRAC_PI_2;

pub fn is_retrograde_inclination(inclination: Angle) -> bool {
    let val = inclination.value();
    val > FRAC_PI_2 && val.is_finite()
}