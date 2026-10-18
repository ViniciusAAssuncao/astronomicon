use super::celestial_body_details::CelestialBodyDetails;
use super::celestial_body_environment::{RadiativeContext, star_flux_at_distance};
use super::celestial_body_facts::push;
use astronomicon_app::climate::circulation::{ReferenceWindInputs, reference_wind_profile};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::{Planet, PlanetKind};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::lagrange::orbital_plane_normal;
use astronomicon_core::math::rotation_state::{
    planet_rotation_axis_direction, solstice_reference_direction,
};
use astronomicon_core::units::{Angle, Irradiance, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::atmosphere_repository;

pub async fn append_reference_winds(
    details: &mut CelestialBodyDetails,
    pool: &SqlitePool,
    catalog: &EphemerisCatalog,
    planet: &Planet,
    context: &RadiativeContext,
) {
    if matches!(planet.kind(), PlanetKind::GasGiant | PlanetKind::IceGiant) {
        return;
    }
    let (
        Some(radius),
        Some(period),
        Some(roughness),
        Some(obliquity),
        Some(solstice),
        Some(elements),
        Some(albedo),
        Some(inertia),
    ) = (
        planet.equatorial_radius(),
        planet.rotation_period(),
        planet.surface_roughness(),
        planet.obliquity(),
        planet.solstice_true_anomaly(),
        planet.orbital_elements(),
        planet.bond_albedo(),
        planet.thermal_inertia(),
    )
    else {
        return;
    };
    let Some(mean_k) = details
        .facts
        .iter()
        .find(|f| f.section == "environment" && f.key == "surface_mean_k")
        .map(|f| f.value)
    else {
        return;
    };
    let Ok(Some(atmosphere)) = atmosphere_repository::get_by_planet_id(pool, &planet.id()).await
    else {
        return;
    };
    if atmosphere.surface_pressure().value() < 1.0 || atmosphere.composition().is_empty() {
        return;
    }
    let Ok(cp) = atmosphere.mean_specific_heat_capacity() else {
        return;
    };
    let gravity = surface_gravity(gravitational_parameter(planet.mass()), radius);
    let Ok(scale_height) = atmosphere.scale_height(gravity, Temperature::new(mean_k)) else {
        return;
    };
    let pole = planet_rotation_axis_direction(
        orbital_plane_normal(&elements),
        obliquity,
        solstice_reference_direction(&elements, solstice),
    );
    let Some(target) = context.positions.get(&planet.id()).copied() else {
        return;
    };
    let mut stars = Vec::with_capacity(catalog.stars().len());
    for star in catalog.stars() {
        let Some(source) = context.positions.get(&star.id()).copied() else {
            return;
        };
        let direction = (source - target).raw();
        let distance = direction.magnitude();
        if !distance.is_finite() || distance <= 0.0 {
            return;
        }
        let Some(flux) = star_flux_at_distance(star, distance) else {
            return;
        };
        let declination = (pole.dot(&(direction / distance))).clamp(-1.0, 1.0).asin();
        if !declination.is_finite() {
            return;
        }
        stars.push((Irradiance::new(flux), Angle::new(declination)));
    }
    let Some(winds) = reference_wind_profile(&ReferenceWindInputs {
        radius,
        gravity,
        rotation_period: period,
        roughness,
        scale_height,
        pressure: atmosphere.surface_pressure(),
        heat_capacity: cp,
        global_temperature: Temperature::new(mean_k),
        greenhouse: atmosphere.greenhouse_effect(),
        albedo,
        thermal_inertia: inertia,
        stars,
    }) else {
        return;
    };
    push(
        &mut details.facts,
        "climate",
        "circulation_cells",
        Some(winds.cells_per_hemisphere as f64),
    );
    push(
        &mut details.facts,
        "climate",
        "thermal_redistribution",
        Some(winds.redistribution_efficiency),
    );
    for band in winds.bands {
        let label = if band.latitude_degrees > 0 {
            format!("{}n", band.latitude_degrees)
        } else {
            format!("{}s", -band.latitude_degrees)
        };
        push(
            &mut details.facts,
            "climate",
            &format!("temperature_{label}"),
            Some(band.temperature_k),
        );
        push(
            &mut details.facts,
            "climate",
            &format!("surface_wind_{label}"),
            Some(band.surface_speed_m_s),
        );
        push(
            &mut details.facts,
            "climate",
            &format!("jet_wind_{label}"),
            Some(band.jet_speed_m_s),
        );
    }
}
