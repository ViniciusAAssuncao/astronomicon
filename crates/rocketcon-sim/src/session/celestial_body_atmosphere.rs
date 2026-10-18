use super::celestial_body_details::CelestialBodyDetails;
use super::celestial_body_facts::push;
use astronomicon_core::domain::Planet;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::units::Temperature;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::atmosphere_repository;

pub async fn append_atmospheric_physics(
    details: &mut CelestialBodyDetails,
    pool: &SqlitePool,
    body: &Planet,
) {
    let Some(radius) = body.equatorial_radius().filter(|r| r.value() > 0.0) else {
        return;
    };
    let Some(temperature) = details
        .facts
        .iter()
        .find(|fact| fact.section == "environment" && fact.key == "surface_mean_k")
        .map(|fact| fact.value)
        .filter(|t| t.is_finite() && *t > 0.0)
    else {
        return;
    };
    let Ok(Some(atmosphere)) = atmosphere_repository::get_by_planet_id(pool, &body.id()).await
    else {
        return;
    };
    if atmosphere.surface_pressure().value() < 1.0 || atmosphere.composition().is_empty() {
        return;
    }
    let temperature = Temperature::new(temperature);
    let gravity = surface_gravity(gravitational_parameter(body.mass()), radius);
    if !gravity.value().is_finite() || gravity.value() <= 0.0 {
        return;
    }
    let mut add = |key: &str, value: f64| {
        if value.is_finite() && value > 0.0 {
            push(&mut details.facts, "atmosphere", key, Some(value));
        }
    };
    if let Ok(molar_mass) = atmosphere.mean_molar_mass() {
        add("mean_molar_mass", molar_mass.value());
    }
    if let Ok(cp) = atmosphere.mean_specific_heat_capacity() {
        add("specific_heat_capacity", cp);
    }
    if let Ok(density) = atmosphere.density_at_surface(temperature) {
        add("surface_air_density", density.value());
    }
    if let Ok(sound) = atmosphere.speed_of_sound_at(temperature) {
        add("surface_sound_speed", sound.value());
    }
    if let Ok(viscosity) = atmosphere.mean_dynamic_viscosity(temperature) {
        add("air_dynamic_viscosity", viscosity.value());
    }
    if let Ok(height) = atmosphere.scale_height(gravity, temperature) {
        add("atmospheric_scale_height", height.value());
    }
    add("atmospheric_mass_column", atmosphere.mass_column(gravity));
}
