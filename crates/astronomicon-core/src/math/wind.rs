use crate::math::rotation::coriolis_parameter;
use crate::units::{
    Acceleration,
    Angle,
    AngularVelocity,
    Duration,
    Length,
    Speed,
    Temperature,
    TemperatureGradient,
};
use std::f64::consts::{ PI, TAU };

pub fn latitudinal_temperature_gradient(
    temp_north: Temperature,
    temp_south: Temperature,
    latitude_north: Angle,
    latitude_south: Angle,
    planet_radius: Length
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
    temp_gradient_y: TemperatureGradient
) -> f64 {
    let g = gravity.value();
    let f = coriolis_f.value();
    let t = mean_temperature.value();
    let dt_dy = temp_gradient_y.value();

    if
        f.abs() <= 1e-12 ||
        t <= 0.0 ||
        !g.is_finite() ||
        !f.is_finite() ||
        !t.is_finite() ||
        !dt_dy.is_finite()
    {
        return 0.0;
    }

    -(g / (f * t)) * dt_dy
}

pub fn thermal_wind_speed(
    gravity: Acceleration,
    coriolis_f: AngularVelocity,
    mean_temperature: Temperature,
    temp_gradient_y: TemperatureGradient,
    height: Length
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
    height: Length
) -> Speed {
    let lat_val = latitude.value();
    let equatorial_limit = (5.0 * PI) / 180.0;

    if lat_val.abs() < equatorial_limit {
        let f_pos = coriolis_parameter(planetary_rotation, Angle::new(equatorial_limit));
        let f_neg = coriolis_parameter(planetary_rotation, Angle::new(-equatorial_limit));

        let u_pos = thermal_wind_speed(
            gravity,
            f_pos,
            mean_temperature,
            temp_gradient_y,
            height
        ).value();
        let u_neg = thermal_wind_speed(
            gravity,
            f_neg,
            mean_temperature,
            temp_gradient_y,
            height
        ).value();

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
    cross_isobar_angle: Angle
) -> (Speed, Speed) {
    let ff = friction_factor.clamp(0.0, 1.0);
    let speed = geostrophic_speed.value() * ff;
    let alpha = cross_isobar_angle.value();

    let u = speed * alpha.cos();
    let v = speed * alpha.sin();

    (Speed::new(u), Speed::new(v))
}

pub fn diurnal_thermal_gradient(
    diurnal_range: Temperature,
    planet_radius: Length,
    latitude: Angle
) -> TemperatureGradient {
    let dt = diurnal_range.value();
    let r = planet_radius.value();
    let phi = latitude.value();

    if dt <= 0.0 || r <= 0.0 || !dt.is_finite() || !r.is_finite() || !phi.is_finite() {
        return TemperatureGradient::new(0.0);
    }

    let cos_phi = phi.cos().abs().max(0.01);
    let dx = (PI * r * cos_phi).max(1000.0);
    TemperatureGradient::new(dt / dx)
}

pub fn diurnal_thermal_wind_speed(
    diurnal_range: Temperature,
    mean_temperature: Temperature,
    gravity: Acceleration,
    scale_height: Length,
    planet_radius: Length,
    rotation_period: Duration,
    latitude: Angle
) -> Speed {
    let dt = diurnal_range.value();
    let t_m = mean_temperature.value();
    let g = gravity.value();
    let h = scale_height.value();
    let r = planet_radius.value();
    let p_rot = rotation_period.value();
    let phi = latitude.value();

    if
        dt <= 0.0 ||
        t_m <= 0.0 ||
        g <= 0.0 ||
        h <= 0.0 ||
        r <= 0.0 ||
        !dt.is_finite() ||
        !t_m.is_finite() ||
        !g.is_finite() ||
        !h.is_finite() ||
        !r.is_finite() ||
        !phi.is_finite()
    {
        return Speed::new(0.0);
    }

    let omega = if p_rot.is_finite() && p_rot > 0.0 { TAU / p_rot } else { 0.0 };

    let f_cor = 2.0 * omega * phi.sin().abs();
    let cos_phi = phi.cos().abs().max(0.01);
    let dx = PI * r * cos_phi;

    let gamma_drag = 1.0e-4;
    let damping_freq = (omega * omega + f_cor * f_cor + gamma_drag * gamma_drag).sqrt();

    let dt_over_t = (dt / t_m).clamp(0.0, 2.0);
    let p_grad_accel = (g * h * dt_over_t) / dx;
    let v_tide = p_grad_accel / damping_freq;

    let pbl_depth = (0.15 * h).clamp(50.0, 3000.0);
    let v_breeze = (2.0 * g * pbl_depth * dt_over_t * (pbl_depth / dx).min(1.0)).sqrt();

    let v_combined = (v_tide * v_tide + v_breeze * v_breeze).sqrt();
    let max_c = (g * h).sqrt();
    let speed = v_combined.min(max_c * dt_over_t);

    if !speed.is_finite() || speed <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(speed)
    }
}

pub fn combined_surface_wind_speed(
    synoptic_surface_speed: Speed,
    diurnal_thermal_speed: Speed
) -> Speed {
    let u_syn = synoptic_surface_speed.value().max(0.0);
    let u_diurnal = diurnal_thermal_speed.value().max(0.0);

    if !u_syn.is_finite() && !u_diurnal.is_finite() {
        return Speed::new(0.0);
    }

    let syn = if u_syn.is_finite() { u_syn } else { 0.0 };
    let diu = if u_diurnal.is_finite() { u_diurnal } else { 0.0 };

    Speed::new((syn * syn + diu * diu).sqrt())
}

pub fn combined_surface_wind_components(
    geostrophic_speed: Speed,
    friction_factor: f64,
    cross_isobar_angle: Angle,
    diurnal_thermal_speed: Speed,
    diurnal_phase: Angle
) -> (Speed, Speed) {
    let (u_syn, v_syn) = surface_wind_components(
        geostrophic_speed,
        friction_factor,
        cross_isobar_angle
    );
    let v_diu = diurnal_thermal_speed.value().max(0.0);
    let phi = diurnal_phase.value();

    let u_diurnal = v_diu * phi.sin();
    let v_diurnal = -v_diu * phi.cos() * 0.2;

    let u_total = u_syn.value() + u_diurnal;
    let v_total = v_syn.value() + v_diurnal;

    (Speed::new(u_total), Speed::new(v_total))
}

pub fn total_surface_wind_velocity(
    geostrophic_speed: Speed,
    friction_factor: f64,
    cross_isobar_angle: Angle,
    diurnal_thermal_speed: Speed,
    diurnal_phase: Angle
) -> Speed {
    let (u, v) = combined_surface_wind_components(
        geostrophic_speed,
        friction_factor,
        cross_isobar_angle,
        diurnal_thermal_speed,
        diurnal_phase
    );
    let u_val = u.value();
    let v_val = v.value();
    Speed::new((u_val * u_val + v_val * v_val).sqrt())
}
