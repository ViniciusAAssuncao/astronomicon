use crate::units::Angle;
use std::f64::consts::PI;

pub fn solar_declination(
    obliquity: Angle,
    _argument_of_periapsis: Angle,
    solstice_true_anomaly: Angle,
    true_anomaly: Angle,
) -> Angle {
    let sin_delta =
        obliquity.value().sin() * (true_anomaly.value() - solstice_true_anomaly.value()).sin();
    Angle::new(sin_delta.clamp(-1.0, 1.0).asin())
}

pub fn day_length_half_angle(latitude: Angle, declination: Angle) -> Angle {
    let val = -latitude.value().tan() * declination.value().tan();
    if val.is_nan() {
        Angle::new(PI / 2.0)
    } else {
        Angle::new(val.clamp(-1.0, 1.0).acos())
    }
}

pub fn mean_daily_insolation_factor(
    latitude: Angle,
    declination: Angle,
    day_length_half_angle: Angle,
) -> f64 {
    let phi = latitude.value();
    let delta = declination.value();
    let h0 = day_length_half_angle.value();

    let factor = (h0 * phi.sin() * delta.sin() + phi.cos() * delta.cos() * h0.sin()) / PI;
    factor.clamp(0.0, 1.0)
}
