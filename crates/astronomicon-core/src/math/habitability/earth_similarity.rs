use crate::math::gravity::gravitational_parameter;
use crate::math::radiometry::{escape_velocity, mean_density};
use crate::units::constants::{
    EARTH_MASS, EARTH_MEAN_SURFACE_TEMPERATURE_K, EARTH_RADIUS, ESI_WEIGHT_DENSITY,
    ESI_WEIGHT_ESCAPE_VELOCITY, ESI_WEIGHT_RADIUS, ESI_WEIGHT_SURFACE_TEMPERATURE,
};
use crate::units::{Density, Length, Mass, Speed, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EarthSimilarityIndex {
    pub interior: f64,
    pub surface: f64,
    pub global: f64,
    pub radius_component: f64,
    pub density_component: f64,
    pub escape_velocity_component: f64,
    pub surface_temperature_component: f64,
}

impl EarthSimilarityIndex {
    pub fn new(
        interior: f64,
        surface: f64,
        global: f64,
        radius_component: f64,
        density_component: f64,
        escape_velocity_component: f64,
        surface_temperature_component: f64,
    ) -> Self {
        Self {
            interior,
            surface,
            global,
            radius_component,
            density_component,
            escape_velocity_component,
            surface_temperature_component,
        }
    }

    pub fn interior(&self) -> f64 {
        self.interior
    }

    pub fn surface(&self) -> f64 {
        self.surface
    }

    pub fn global(&self) -> f64 {
        self.global
    }

    pub fn radius_component(&self) -> f64 {
        self.radius_component
    }

    pub fn density_component(&self) -> f64 {
        self.density_component
    }

    pub fn escape_velocity_component(&self) -> f64 {
        self.escape_velocity_component
    }

    pub fn surface_temperature_component(&self) -> f64 {
        self.surface_temperature_component
    }
}

pub fn earth_reference_radius() -> Length {
    Length::new(EARTH_RADIUS)
}

pub fn earth_reference_density() -> Density {
    mean_density(Mass::new(EARTH_MASS), Length::new(EARTH_RADIUS))
}

pub fn earth_reference_escape_velocity() -> Speed {
    let mu = gravitational_parameter(Mass::new(EARTH_MASS));
    escape_velocity(mu, Length::new(EARTH_RADIUS))
}

pub fn earth_reference_surface_temperature() -> Temperature {
    Temperature::new(EARTH_MEAN_SURFACE_TEMPERATURE_K)
}

pub fn property_similarity(value: f64, reference: f64, weight: f64, n_properties: f64) -> f64 {
    if value <= 0.0
        || reference <= 0.0
        || weight <= 0.0
        || n_properties <= 0.0
        || !value.is_finite()
        || !reference.is_finite()
        || !weight.is_finite()
        || !n_properties.is_finite()
    {
        return 0.0;
    }

    let diff = (value - reference).abs();
    let sum = value + reference;

    if sum <= 0.0 {
        return 0.0;
    }

    let base = (1.0 - diff / sum).clamp(0.0, 1.0);
    let exponent = weight / n_properties;
    let result = base.powf(exponent);

    if !result.is_finite() || result < 0.0 {
        0.0
    } else {
        result.clamp(0.0, 1.0)
    }
}

pub fn radius_similarity(radius: Length) -> f64 {
    property_similarity(
        radius.value(),
        earth_reference_radius().value(),
        ESI_WEIGHT_RADIUS,
        1.0,
    )
}

pub fn density_similarity(density: Density) -> f64 {
    property_similarity(
        density.value(),
        earth_reference_density().value(),
        ESI_WEIGHT_DENSITY,
        1.0,
    )
}

pub fn escape_velocity_similarity(escape_velocity: Speed) -> f64 {
    property_similarity(
        escape_velocity.value(),
        earth_reference_escape_velocity().value(),
        ESI_WEIGHT_ESCAPE_VELOCITY,
        1.0,
    )
}

pub fn surface_temperature_similarity(surface_temperature: Temperature) -> f64 {
    property_similarity(
        surface_temperature.value(),
        earth_reference_surface_temperature().value(),
        ESI_WEIGHT_SURFACE_TEMPERATURE,
        1.0,
    )
}

pub fn earth_similarity_interior(mean_radius: Length, bulk_density: Density) -> f64 {
    let s_r = radius_similarity(mean_radius);
    let s_rho = density_similarity(bulk_density);
    (s_r * s_rho).max(0.0).sqrt().clamp(0.0, 1.0)
}

pub fn earth_similarity_surface(
    escape_velocity: Speed,
    surface_temperature: Temperature,
) -> f64 {
    let s_v = escape_velocity_similarity(escape_velocity);
    let s_t = surface_temperature_similarity(surface_temperature);
    (s_v * s_t).max(0.0).sqrt().clamp(0.0, 1.0)
}

pub fn earth_similarity_from_sub_indices(esi_interior: f64, esi_surface: f64) -> f64 {
    let interior = esi_interior.clamp(0.0, 1.0);
    let surface = esi_surface.clamp(0.0, 1.0);
    (interior * surface).max(0.0).sqrt().clamp(0.0, 1.0)
}

pub fn global_earth_similarity(
    mean_radius: Length,
    bulk_density: Density,
    escape_velocity: Speed,
    surface_temperature: Temperature,
) -> f64 {
    let esi_int = earth_similarity_interior(mean_radius, bulk_density);
    let esi_surf = earth_similarity_surface(escape_velocity, surface_temperature);
    earth_similarity_from_sub_indices(esi_int, esi_surf)
}

pub fn calculate_earth_similarity_index(
    mean_radius: Length,
    bulk_density: Density,
    escape_velocity: Speed,
    surface_temperature: Temperature,
) -> EarthSimilarityIndex {
    let s_r = radius_similarity(mean_radius);
    let s_rho = density_similarity(bulk_density);
    let s_v = escape_velocity_similarity(escape_velocity);
    let s_t = surface_temperature_similarity(surface_temperature);

    let interior = (s_r * s_rho).max(0.0).sqrt().clamp(0.0, 1.0);
    let surface = (s_v * s_t).max(0.0).sqrt().clamp(0.0, 1.0);
    let global = (interior * surface).max(0.0).sqrt().clamp(0.0, 1.0);

    EarthSimilarityIndex {
        interior,
        surface,
        global,
        radius_component: s_r,
        density_component: s_rho,
        escape_velocity_component: s_v,
        surface_temperature_component: s_t,
    }
}