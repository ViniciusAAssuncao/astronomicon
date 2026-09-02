use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{
    Angle, Density, Duration, Irradiance, Length, Temperature, TemperatureGradient,
};
use std::f64::consts::PI;

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

pub fn thermal_inertia_from_properties(
    thermal_conductivity: f64,
    density: Density,
    specific_heat_capacity: f64,
) -> f64 {
    let rho = density.value();
    if thermal_conductivity <= 0.0
        || rho <= 0.0
        || specific_heat_capacity <= 0.0
        || !thermal_conductivity.is_finite()
        || !rho.is_finite()
        || !specific_heat_capacity.is_finite()
    {
        0.0
    } else {
        (thermal_conductivity * rho * specific_heat_capacity).sqrt()
    }
}

pub fn thermal_inertia_from_factor(thermal_inertia_factor: f64) -> f64 {
    let f = thermal_inertia_factor.clamp(0.0, 1.0);
    if !f.is_finite() {
        return 50.0;
    }
    20.0 + f * 2500.0
}

pub fn diurnal_temperature_range(
    local_insolation: Irradiance,
    bond_albedo: f64,
    rotation_period: Duration,
    thermal_inertia_si: f64,
    column_heat_capacity: f64,
    mean_temperature: Temperature,
) -> Temperature {
    let f_in = local_insolation.value();
    let t_m = mean_temperature.value();
    let p_rot = rotation_period.value();

    if f_in <= 0.0 || t_m <= 0.0 || !f_in.is_finite() || !t_m.is_finite() {
        return Temperature::new(0.0);
    }

    let a = bond_albedo.clamp(0.0, 1.0);
    let f_abs = (1.0 - a) * f_in;
    if f_abs <= 0.0 {
        return Temperature::new(0.0);
    }

    let omega = if p_rot.is_finite() && p_rot > 0.0 {
        (2.0 * PI) / p_rot
    } else {
        0.0
    };

    let gamma = if thermal_inertia_si.is_finite() && thermal_inertia_si > 0.0 {
        thermal_inertia_si
    } else {
        50.0
    };

    let c_col = if column_heat_capacity.is_finite() && column_heat_capacity > 0.0 {
        column_heat_capacity
    } else {
        0.0
    };

    let y_ground = gamma * omega.sqrt();
    let y_atm = c_col * omega;
    let y_rad = 4.0 * STEFAN_BOLTZMANN_CONSTANT * t_m.powi(3);

    let y_total = y_ground + y_atm + y_rad;
    if y_total <= 0.0 || !y_total.is_finite() {
        return Temperature::new(0.0);
    }

    let delta_t = f_abs / y_total;
    let max_possible =
        local_radiative_equilibrium_temperature(local_insolation, bond_albedo).value();
    let clamped_delta_t = delta_t.clamp(0.0, max_possible.max(t_m * 2.0));

    Temperature::new(clamped_delta_t)
}

pub fn diurnal_temperature_range_simple(
    local_insolation: Irradiance,
    bond_albedo: f64,
    rotation_period: Duration,
    thermal_inertia_factor: f64,
    column_heat_capacity: f64,
    mean_temperature: Temperature,
) -> Temperature {
    let gamma = thermal_inertia_from_factor(thermal_inertia_factor);
    diurnal_temperature_range(
        local_insolation,
        bond_albedo,
        rotation_period,
        gamma,
        column_heat_capacity,
        mean_temperature,
    )
}

pub fn diurnal_temperature_extrema(
    mean_temperature: Temperature,
    diurnal_range: Temperature,
) -> (Temperature, Temperature) {
    let t_m = mean_temperature.value();
    let dt = diurnal_range.value();
    let half_dt = 0.5 * dt.max(0.0);
    let t_max = t_m + half_dt;
    let t_min = (t_m - half_dt).max(0.0);
    (Temperature::new(t_max), Temperature::new(t_min))
}

pub fn diurnal_temperature_at_phase(
    mean_temperature: Temperature,
    diurnal_range: Temperature,
    diurnal_phase: Angle,
) -> Temperature {
    let t_m = mean_temperature.value();
    let dt = diurnal_range.value();
    let phi = diurnal_phase.value();
    let t = t_m + 0.5 * dt * phi.cos();
    Temperature::new(t.max(0.0))
}