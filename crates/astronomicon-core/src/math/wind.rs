use crate::math::rotation::coriolis_parameter;
use crate::units::{Acceleration, Angle, AngularVelocity, Length, Speed, Temperature, TemperatureGradient};
use std::f64::consts::PI;

pub fn latitudinal_temperature_gradient(
    temp_north: Temperature,
    temp_south: Temperature,
    latitude_north: Angle,
    latitude_south: Angle,
    planet_radius: Length,
) -> TemperatureGradient {
    let r = planet_radius.value();
    let d_phi = latitude_north.value() - latitude_south.value();
    let dy = r * d_phi;

    if dy.abs() <= 1e-6 || !dy.is_finite() {
        return TemperatureGradient::new(0.0);
    }

    let dt = temp_north.value() - temp_south.value();
    if !dt.is_finite() {
        return TemperatureGradient::new(0.0);
    }

    TemperatureGradient::new(dt / dy)
}

pub fn thermal_wind_shear(
    gravity: Acceleration,
    coriolis_f: AngularVelocity,
    mean_temperature: Temperature,
    temp_gradient_y: TemperatureGradient,
) -> f64 {
    let g = gravity.value();
    let f = coriolis_f.value();
    let t = mean_temperature.value();
    let dt_dy = temp_gradient_y.value();

    if f.abs() <= 1e-12 || t <= 0.0 || !g.is_finite() || !f.is_finite() || !t.is_finite() || !dt_dy.is_finite() {
        return 0.0;
    }

    -(g / (f * t)) * dt_dy
}

pub fn thermal_wind_speed(
    gravity: Acceleration,
    coriolis_f: AngularVelocity,
    mean_temperature: Temperature,
    temp_gradient_y: TemperatureGradient,
    height: Length,
) -> Speed {
    let shear = thermal_wind_shear(gravity, coriolis_f, mean_temperature, temp_gradient_y);
    let h = height.value();

    if !h.is_finite() || h <= 0.0 {
        return Speed::new(0.0);
    }

    Speed::new(shear * h)
}

pub fn zonal_jet_stream_speed(
    gravity: Acceleration,
    planetary_rotation: AngularVelocity,
    _planet_radius: Length,
    latitude: Angle,
    mean_temperature: Temperature,
    temp_gradient_y: TemperatureGradient,
    height: Length,
) -> Speed {
    let lat_val = latitude.value();
    let equatorial_limit = 5.0 * PI / 180.0;

    if lat_val.abs() < equatorial_limit {
        let f_pos = coriolis_parameter(planetary_rotation, Angle::new(equatorial_limit));
        let f_neg = coriolis_parameter(planetary_rotation, Angle::new(-equatorial_limit));

        let u_pos = thermal_wind_speed(gravity, f_pos, mean_temperature, temp_gradient_y, height).value();
        let u_neg = thermal_wind_speed(gravity, f_neg, mean_temperature, temp_gradient_y, height).value();

        let factor = (lat_val + equatorial_limit) / (2.0 * equatorial_limit);
        let interpolated = u_neg + factor * (u_pos - u_neg);

        Speed::new(interpolated)
    } else {
        let f = coriolis_parameter(planetary_rotation, latitude);
        thermal_wind_speed(gravity, f, mean_temperature, temp_gradient_y, height)
    }
}

pub fn surface_wind_speed(geostrophic_speed: Speed, friction_factor: f64) -> Speed {
    let ff = friction_factor.clamp(0.0, 1.0);
    Speed::new(geostrophic_speed.value() * ff)
}

pub fn surface_wind_components(
    geostrophic_speed: Speed,
    friction_factor: f64,
    cross_isobar_angle: Angle,
) -> (Speed, Speed) {
    let ff = friction_factor.clamp(0.0, 1.0);
    let speed = geostrophic_speed.value() * ff;
    let alpha = cross_isobar_angle.value();

    let u = speed * alpha.cos();
    let v = speed * alpha.sin();

    (Speed::new(u), Speed::new(v))
}
