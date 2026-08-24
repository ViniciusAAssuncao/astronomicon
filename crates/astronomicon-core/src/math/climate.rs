use crate::math::thermodynamics::MatterState;
use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{
    Acceleration, Angle, Density, Irradiance, Length, Pressure, Temperature, TemperatureGradient,
};
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

pub fn oceanic_column_heat_capacity(
    liquid_depth: Length,
    liquid_density: Density,
    specific_heat_capacity: f64,
) -> f64 {
    let d = liquid_depth.value();
    let rho = liquid_density.value();
    let cp = specific_heat_capacity;

    if d <= 0.0 || rho <= 0.0 || cp <= 0.0 || !d.is_finite() || !rho.is_finite() || !cp.is_finite()
    {
        return 0.0;
    }

    d * rho * cp
}

pub fn combined_column_heat_capacity(
    atmospheric_column_heat_capacity: f64,
    oceanic_column_heat_capacity: f64,
    ocean_coverage_fraction: f64,
) -> f64 {
    let c_atm =
        if atmospheric_column_heat_capacity.is_finite() && atmospheric_column_heat_capacity > 0.0 {
            atmospheric_column_heat_capacity
        } else {
            0.0
        };
    let c_oce = if oceanic_column_heat_capacity.is_finite() && oceanic_column_heat_capacity > 0.0 {
        oceanic_column_heat_capacity
    } else {
        0.0
    };
    let cov = ocean_coverage_fraction.clamp(0.0, 1.0);

    c_atm + cov * c_oce
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

pub fn combined_thermal_redistribution_efficiency(
    atmospheric_heat_capacity: f64,
    oceanic_heat_capacity: f64,
    ocean_coverage_fraction: f64,
    circulation_cells_per_hemisphere: u32,
) -> f64 {
    let combined_heat_capacity = combined_column_heat_capacity(
        atmospheric_heat_capacity,
        oceanic_heat_capacity,
        ocean_coverage_fraction,
    );

    if combined_heat_capacity <= 0.0 || !combined_heat_capacity.is_finite() {
        return 0.0;
    }

    let n_cells = circulation_cells_per_hemisphere.max(1) as f64;
    let ref_heat_capacity = 2.5e6;
    let ocean_cov = ocean_coverage_fraction.clamp(0.0, 1.0);
    let effective_cells = n_cells * (1.0 - 0.7 * ocean_cov);
    let mass_buffering = combined_heat_capacity / (combined_heat_capacity + ref_heat_capacity);

    (mass_buffering / effective_cells.max(1.0)).clamp(0.0, 1.0)
}

pub fn dynamic_surface_albedo(
    base_albedo: f64,
    hydrosphere_state: MatterState,
    surface_coverage_fraction: f64,
    liquid_albedo: f64,
    ice_albedo: f64,
) -> f64 {
    let base = base_albedo.clamp(0.0, 1.0);
    let cov = surface_coverage_fraction.clamp(0.0, 1.0);

    if cov <= 0.0 {
        return base;
    }

    let hydro_albedo = match hydrosphere_state {
        MatterState::Solid => ice_albedo.clamp(0.0, 1.0),
        MatterState::Liquid => liquid_albedo.clamp(0.0, 1.0),
        MatterState::Vapor | MatterState::Supercritical => base,
    };

    (1.0 - cov) * base + cov * hydro_albedo
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
