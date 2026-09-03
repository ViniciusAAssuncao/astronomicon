use super::types::ThermalNode;
use astronomicon_core::math::radiometry::graybody_radiation::net_graybody_radiated_power;
use astronomicon_core::units::{Irradiance, Luminosity, Temperature, Vector3};
use std::f64::consts::PI;

pub fn node_net_radiation(
    node: &ThermalNode,
    environment_temperature: Temperature,
) -> Luminosity {
    if node.exposed_area_m2 <= 0.0 || !node.exposed_area_m2.is_finite() {
        return Luminosity::new(0.0);
    }
    net_graybody_radiated_power(
        node.emissivity,
        node.exposed_area_m2,
        node.temperature,
        environment_temperature,
    )
}

pub fn node_solar_heat_gain(
    node: &ThermalNode,
    solar_irradiance: Irradiance,
    sun_direction_body: Vector3,
    is_eclipsed: bool,
) -> Luminosity {
    if is_eclipsed || node.exposed_area_m2 <= 0.0 || solar_irradiance.value() <= 0.0 {
        return Luminosity::new(0.0);
    }

    let irr = solar_irradiance.value();
    let alpha = node.solar_absorptivity.clamp(0.0, 1.0);
    let s_hat = sun_direction_body.normalized();

    let incidence = if s_hat.magnitude() < 1e-6 {
        0.5
    } else {
        (s_hat.0.abs() + s_hat.1.abs() + s_hat.2.abs()).clamp(0.1, 1.0)
    };

    let projected_area = node.exposed_area_m2 / PI;
    let power = alpha * irr * projected_area * incidence;

    if !power.is_finite() || power < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(power)
    }
}
