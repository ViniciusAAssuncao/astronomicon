use astronomicon_core::math::circulation::{circulation_cells_per_hemisphere, rhines_scale};
use astronomicon_core::math::climate::{
    advective_local_temperature, atmospheric_column_heat_capacity, blended_local_temperature,
    day_length_half_angle, local_radiative_equilibrium_temperature, mean_daily_insolation_factor,
    thermal_redistribution_efficiency,
};
use astronomicon_core::math::rotation::{
    angular_velocity_from_rotation_period, rossby_beta_parameter,
};
use astronomicon_core::math::wind::{latitudinal_temperature_gradient, zonal_jet_stream_speed};
use astronomicon_core::units::{
    Acceleration, Angle, Duration, Irradiance, Length, Pressure, Speed, Temperature,
};
use std::f64::consts::PI;

pub struct ReferenceWindInputs {
    pub radius: Length,
    pub gravity: Acceleration,
    pub rotation_period: Duration,
    pub roughness: Length,
    pub scale_height: Length,
    pub pressure: Pressure,
    pub heat_capacity: f64,
    pub global_temperature: Temperature,
    pub greenhouse: Temperature,
    pub albedo: f64,
    pub thermal_inertia: f64,
    pub stars: Vec<(Irradiance, Angle)>,
}

pub struct ReferenceWindBand {
    pub latitude_degrees: i32,
    pub temperature_k: f64,
    pub surface_speed_m_s: f64,
    pub jet_speed_m_s: f64,
}

pub struct ReferenceWindProfile {
    pub cells_per_hemisphere: u32,
    pub redistribution_efficiency: f64,
    pub bands: Vec<ReferenceWindBand>,
}

fn insolation(inputs: &ReferenceWindInputs, phi: f64) -> f64 {
    inputs
        .stars
        .iter()
        .map(|(flux, declination)| {
            let half_day = day_length_half_angle(Angle::new(phi), *declination);
            flux.value() * mean_daily_insolation_factor(Angle::new(phi), *declination, half_day)
        })
        .sum()
}

fn radiative_temperature(inputs: &ReferenceWindInputs, phi: f64) -> Temperature {
    let absorbed = insolation(inputs, phi);
    let local = local_radiative_equilibrium_temperature(Irradiance::new(absorbed), inputs.albedo)
        + inputs.greenhouse;
    blended_local_temperature(inputs.global_temperature, local, inputs.thermal_inertia)
}

pub fn reference_wind_profile(inputs: &ReferenceWindInputs) -> Option<ReferenceWindProfile> {
    let r = inputs.radius.value();
    let g = inputs.gravity.value();
    let period = inputs.rotation_period.value();
    let h = inputs.scale_height.value();
    let z0 = inputs.roughness.value();
    let t = inputs.global_temperature.value();
    if ![
        r,
        g,
        period,
        h,
        z0,
        t,
        inputs.pressure.value(),
        inputs.heat_capacity,
        inputs.greenhouse.value(),
        inputs.albedo,
        inputs.thermal_inertia,
    ]
    .iter()
    .all(|v| v.is_finite())
        || r <= 0.0
        || g <= 0.0
        || period <= 0.0
        || h <= 10.0
        || z0 <= 0.0
        || z0 >= 10.0
        || t <= 0.0
        || inputs.pressure.value() < 1.0
        || inputs.heat_capacity <= 0.0
        || !(0.0..=1.0).contains(&inputs.albedo)
        || !(0.0..=1.0).contains(&inputs.thermal_inertia)
        || inputs.stars.is_empty()
        || inputs
            .stars
            .iter()
            .any(|(f, d)| !f.value().is_finite() || f.value() <= 0.0 || !d.value().is_finite())
    {
        return None;
    }
    let omega = angular_velocity_from_rotation_period(inputs.rotation_period);
    let beta = rossby_beta_parameter(omega, Angle::new(0.0), inputs.radius);
    if !beta.is_finite() || beta <= 0.0 {
        return None;
    }
    let equator = radiative_temperature(inputs, 0.0).value();
    let north_pole = radiative_temperature(inputs, PI / 2.0).value();
    let south_pole = radiative_temperature(inputs, -PI / 2.0).value();
    let delta = ((equator - north_pole).abs() + (equator - south_pole).abs()) / 2.0;
    let characteristic = Speed::new((g * h * delta / t).sqrt());
    if !characteristic.value().is_finite() || characteristic.value() <= 0.0 {
        return None;
    }
    let rhines = rhines_scale(characteristic, beta);
    let cells = circulation_cells_per_hemisphere(inputs.radius, rhines);
    let column =
        atmospheric_column_heat_capacity(inputs.pressure, inputs.gravity, inputs.heat_capacity);
    let redistribution = thermal_redistribution_efficiency(column, cells);
    if !redistribution.is_finite() {
        return None;
    }
    let temperature = |phi: f64| {
        advective_local_temperature(
            inputs.global_temperature,
            radiative_temperature(inputs, phi),
            redistribution,
        )
    };
    let pbl_height = (0.15 * h).clamp(100.0, 5000.0);
    if pbl_height <= z0 {
        return None;
    }
    let friction = ((10.0 / z0).ln() / (pbl_height / z0).ln()).clamp(0.0, 1.0);
    let mut bands = Vec::with_capacity(4);
    for degrees in [-60, -30, 30, 60] {
        let phi = (degrees as f64).to_radians();
        let offset = PI / 180.0;
        let gradient = latitudinal_temperature_gradient(
            temperature((phi + offset).min(PI / 2.0)),
            temperature((phi - offset).max(-PI / 2.0)),
            Angle::new(phi + offset),
            Angle::new(phi - offset),
            inputs.radius,
        );
        let local = temperature(phi);
        let jet = zonal_jet_stream_speed(
            inputs.gravity,
            omega,
            inputs.radius,
            Angle::new(phi),
            local,
            gradient,
            inputs.scale_height,
        )
        .value();
        let surface = jet * friction;
        if ![local.value(), jet, surface].iter().all(|v| v.is_finite()) || local.value() <= 0.0 {
            return None;
        }
        bands.push(ReferenceWindBand {
            latitude_degrees: degrees,
            temperature_k: local.value(),
            surface_speed_m_s: surface.abs(),
            jet_speed_m_s: jet.abs(),
        });
    }
    Some(ReferenceWindProfile {
        cells_per_hemisphere: cells,
        redistribution_efficiency: redistribution,
        bands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additive_stellar_flux_and_missing_rotation() {
        let mut inputs = ReferenceWindInputs {
            radius: Length::new(6.3e6),
            gravity: Acceleration::new(9.8),
            rotation_period: Duration::new(86400.0),
            roughness: Length::new(0.01),
            scale_height: Length::new(8500.0),
            pressure: Pressure::new(101325.0),
            heat_capacity: 1000.0,
            global_temperature: Temperature::new(290.0),
            greenhouse: Temperature::new(30.0),
            albedo: 0.3,
            thermal_inertia: 0.3,
            stars: vec![(Irradiance::new(800.0), Angle::new(0.2))],
        };
        let before = reference_wind_profile(&inputs).unwrap();
        inputs
            .stars
            .push((Irradiance::new(600.0), Angle::new(-0.2)));
        let after = reference_wind_profile(&inputs).unwrap();
        assert_eq!(before.bands.len(), 4);
        assert!(after.bands.iter().all(|b| b.surface_speed_m_s.is_finite()));
        assert!(after.bands[2].temperature_k > before.bands[2].temperature_k);
        inputs.rotation_period = Duration::new(0.0);
        assert!(reference_wind_profile(&inputs).is_none());
    }
}
