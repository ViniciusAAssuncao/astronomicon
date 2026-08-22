use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{Acceleration, Angle, Irradiance, Length, Pressure, Temperature, TemperatureGradient};
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

pub fn local_radiative_equilibrium_temperature(
    local_insolation: Irradiance,
    bond_albedo: f64,
) -> Temperature {
    if local_insolation.value() <= 0.0 {
        return Temperature::new(0.0);
    }
    let absorbed = (1.0 - bond_albedo.clamp(0.0, 1.0)) * local_insolation.value();
    let t4 = absorbed / STEFAN_BOLTZMANN_CONSTANT;
    Temperature::new(t4.max(0.0).powf(0.25))
}

pub fn blended_local_temperature(
    global_mean: Temperature,
    local_equilibrium: Temperature,
    thermal_inertia: f64,
) -> Temperature {
    let ti = thermal_inertia.clamp(0.0, 1.0);
    Temperature::new(ti * global_mean.value() + (1.0 - ti) * local_equilibrium.value())
}

pub fn atmospheric_column_heat_capacity(
    surface_pressure: Pressure,
    surface_gravity: Acceleration,
    specific_heat_capacity: f64,
) -> f64 {
    let p = surface_pressure.value();
    let g = surface_gravity.value();
    let cp = specific_heat_capacity;

    if p <= 0.0 || g <= 0.0 || cp <= 0.0 || !p.is_finite() || !g.is_finite() || !cp.is_finite() {
        return 0.0;
    }

    (p / g) * cp
}

pub fn thermal_redistribution_efficiency(
    column_heat_capacity: f64,
    circulation_cells_per_hemisphere: u32,
) -> f64 {
    if column_heat_capacity <= 0.0 || !column_heat_capacity.is_finite() {
        return 0.0;
    }

    let n_cells = circulation_cells_per_hemisphere.max(1) as f64;
    let ref_heat_capacity = 2.5e6;
    let mass_buffering = column_heat_capacity / (column_heat_capacity + ref_heat_capacity);

    (mass_buffering / n_cells).clamp(0.0, 1.0)
}

pub fn advective_local_temperature(
    global_mean: Temperature,
    local_equilibrium: Temperature,
    redistribution_efficiency: f64,
) -> Temperature {
    let eta = redistribution_efficiency.clamp(0.0, 1.0);
    Temperature::new(eta * global_mean.value() + (1.0 - eta) * local_equilibrium.value())
}

pub fn temperature_at_altitude(
    surface_temperature: Temperature,
    altitude: Length,
    lapse_rate: TemperatureGradient,
) -> Temperature {
    let t = surface_temperature.value() - lapse_rate.value() * altitude.value();
    Temperature::new(t.max(0.0))
}
