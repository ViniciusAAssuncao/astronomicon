use super::celestial_body_facts::{self, BodyFact, BodyMaterial, BodyTextFact};
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::{OrbitalElements, OrbitalParent, PlanetKind};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::units::{Length, Mass};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, lithosphere_repository, system_repository,
};
use std::f64::consts::{PI, TAU};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct BodyComposition {
    pub formula: String,
    pub percentage: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodyAtmosphere {
    pub pressure_pa: f64,
    pub gases: Vec<BodyComposition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BodyHydrosphere {
    pub coverage_percent: f64,
    pub depth_m: f64,
    pub components: Vec<BodyComposition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CelestialBodyDetails {
    pub id: Uuid,
    pub gravity_m_s2: Option<f64>,
    pub density_kg_m3: Option<f64>,
    pub rotation_seconds: Option<f64>,
    pub orbital_distance_m: Option<f64>,
    pub orbital_period_seconds: Option<f64>,
    pub eccentricity: Option<f64>,
    pub inclination_deg: Option<f64>,
    pub temperature_k: Option<f64>,
    pub reference_radius_gravity: bool,
    pub atmosphere: Option<BodyAtmosphere>,
    pub hydrosphere: Option<BodyHydrosphere>,
    pub facts: Vec<BodyFact>,
    pub text_facts: Vec<BodyTextFact>,
    pub materials: Vec<BodyMaterial>,
}

fn parent_id(parent: OrbitalParent) -> Option<Uuid> {
    match parent {
        OrbitalParent::Star(id)
        | OrbitalParent::Planet(id)
        | OrbitalParent::Barycenter(id)
        | OrbitalParent::MinorPlanet(id) => Some(id),
        OrbitalParent::Fixed => None,
    }
}

fn valid(value: Option<f64>) -> Option<f64> {
    value.filter(|number| number.is_finite() && *number > 0.0)
}

fn new_details(
    id: Uuid,
    mass: Option<Mass>,
    radius: Option<Length>,
    period: Option<f64>,
    orbit: Option<OrbitalElements>,
    parent: OrbitalParent,
    catalog: &EphemerisCatalog,
) -> CelestialBodyDetails {
    let radius_m = valid(radius.map(|value| value.value()));
    let mass_kg = valid(mass.map(|value| value.value()));
    let gravity_m_s2 = mass.zip(radius).and_then(|(mass, radius)| {
        valid(Some(
            surface_gravity(gravitational_parameter(mass), radius).value(),
        ))
    });
    let density_kg_m3 = mass_kg
        .zip(radius_m)
        .and_then(|(m, r)| valid(Some(m / ((4.0 / 3.0) * PI * r * r * r))));
    let orbital_distance_m = orbit
        .as_ref()
        .and_then(|o| valid(Some(o.semi_major_axis().value())));
    let orbital_period_seconds = orbit
        .as_ref()
        .and_then(|o| {
            let primary = parent_id(parent).and_then(|id| catalog.mass_of_body(id))?;
            let central = mass.unwrap_or_else(|| Mass::new(0.0)) + primary;
            orbital_period(o.semi_major_axis(), gravitational_parameter(central))
        })
        .and_then(|value| valid(Some(value.value())));
    let eccentricity = orbit
        .as_ref()
        .map(|o| o.eccentricity())
        .filter(|number| number.is_finite() && *number >= 0.0);
    let inclination_deg = orbit
        .as_ref()
        .map(|o| o.inclination().value() * 360.0 / TAU)
        .filter(|number| number.is_finite());
    CelestialBodyDetails {
        id,
        gravity_m_s2,
        density_kg_m3,
        rotation_seconds: valid(period),
        orbital_distance_m,
        orbital_period_seconds,
        eccentricity,
        inclination_deg,
        temperature_k: None,
        reference_radius_gravity: false,
        atmosphere: None,
        hydrosphere: None,
        facts: Vec::new(),
        text_facts: Vec::new(),
        materials: Vec::new(),
    }
}

async fn append_system(
    details: &mut CelestialBodyDetails,
    pool: &SqlitePool,
    system_id: Option<Uuid>,
) -> RocketResult<()> {
    if let Some(id) = system_id {
        if let Some(row) = system_repository::get_by_id(pool, &id).await? {
            if !row.name.is_empty() {
                details.text_facts.push(BodyTextFact {
                    section: "general".into(),
                    key: "system_name".into(),
                    value: row.name,
                });
            }
            celestial_body_facts::push(
                &mut details.facts,
                "system",
                "system_distance",
                row.distance_from_sol_m,
            );
            celestial_body_facts::push(
                &mut details.facts,
                "system",
                "right_ascension",
                row.right_ascension_rad.map(f64::to_degrees),
            );
            celestial_body_facts::push(
                &mut details.facts,
                "system",
                "declination",
                row.declination_rad.map(f64::to_degrees),
            );
        }
    }
    Ok(())
}

pub async fn selected_body_details(
    pool: &SqlitePool,
    catalog: &EphemerisCatalog,
    id: Uuid,
) -> RocketResult<CelestialBodyDetails> {
    if let Some(body) = catalog.stars().iter().find(|body| body.id() == id) {
        let mut details = new_details(
            id,
            Some(body.mass()),
            body.radius(),
            body.rotation_period().map(|v| v.value()),
            body.orbital_elements(),
            body.orbital_parent(),
            catalog,
        );
        details.temperature_k = valid(body.effective_temperature().map(|v| v.value()));
        if body.kind() == astronomicon_core::domain::StarKind::BlackHole {
            details.gravity_m_s2 = None;
            details.density_kg_m3 = None;
        }
        celestial_body_facts::star(&mut details.facts, &mut details.text_facts, body);
        append_system(&mut details, pool, body.star_system_id()).await?;
        return Ok(details);
    }
    if let Some(body) = catalog.planets().iter().find(|body| body.id() == id) {
        let giant = matches!(body.kind(), PlanetKind::GasGiant | PlanetKind::IceGiant);
        let mut details = new_details(
            id,
            Some(body.mass()),
            body.equatorial_radius().or(body.polar_radius()),
            body.rotation_period().map(|v| v.value()),
            body.orbital_elements(),
            body.orbital_parent(),
            catalog,
        );
        details.reference_radius_gravity = giant;
        if let (Some(equatorial), Some(polar)) = (body.equatorial_radius(), body.polar_radius()) {
            let volume = (4.0 / 3.0) * PI * equatorial.value().powi(2) * polar.value();
            details.density_kg_m3 = valid(Some(body.mass().value() / volume));
        }
        celestial_body_facts::planet(&mut details.facts, &mut details.text_facts, body);
        append_system(&mut details, pool, body.star_system_id()).await?;
        if let Some(atmosphere) = atmosphere_repository::get_by_planet_id(pool, &id).await? {
            let pressure = atmosphere.surface_pressure().value();
            if pressure.is_finite() && pressure >= 1.0 {
                celestial_body_facts::push(
                    &mut details.facts,
                    "atmosphere",
                    "greenhouse_effect",
                    Some(atmosphere.greenhouse_effect().value()),
                );
                celestial_body_facts::push(
                    &mut details.facts,
                    "atmosphere",
                    "lapse_rate",
                    Some(atmosphere.lapse_rate().value()),
                );
                celestial_body_facts::push(
                    &mut details.facts,
                    "atmosphere",
                    "humidity",
                    atmosphere.surface_humidity(),
                );
                celestial_body_facts::push(
                    &mut details.facts,
                    "atmosphere",
                    "cloud_cover",
                    atmosphere.cloud_coverage_fraction(),
                );
                celestial_body_facts::push(
                    &mut details.facts,
                    "atmosphere",
                    "cloud_nuclei",
                    atmosphere.cloud_condensation_nuclei_factor(),
                );
                details.atmosphere = Some(BodyAtmosphere {
                    pressure_pa: pressure,
                    gases: atmosphere
                        .composition()
                        .iter()
                        .filter(|gas| gas.percentage().is_finite() && gas.percentage() > 0.0)
                        .map(|gas| BodyComposition {
                            formula: gas.formula().to_owned(),
                            percentage: gas.percentage(),
                        })
                        .collect(),
                });
            }
        }
        if !giant {
            if let Some(hydro) = hydrosphere_repository::get_by_planet_id(pool, &id).await? {
                let coverage = hydro.surface_coverage_fraction() * 100.0;
                let depth = hydro.average_depth().value();
                if (coverage > 0.0 || depth > 0.0) && coverage.is_finite() && depth.is_finite() {
                    celestial_body_facts::push(
                        &mut details.facts,
                        "hydrosphere",
                        "salinity",
                        Some(hydro.salinity_or_solute_mass_fraction()),
                    );
                    details.hydrosphere = Some(BodyHydrosphere {
                        coverage_percent: coverage,
                        depth_m: depth,
                        components: hydro
                            .composition()
                            .iter()
                            .filter(|component| {
                                component.percentage().is_finite() && component.percentage() > 0.0
                            })
                            .map(|component| BodyComposition {
                                formula: component.formula().to_owned(),
                                percentage: component.percentage(),
                            })
                            .collect(),
                    });
                }
            }
        }
        if !giant {
            if let Some(rocks) = lithosphere_repository::get_by_planet_id(pool, &id).await? {
                details.materials = rocks
                    .components()
                    .iter()
                    .map(|item| BodyMaterial {
                        name: item.material().name().to_owned(),
                        percentage: item.percentage(),
                    })
                    .collect();
            }
        }
        return Ok(details);
    }
    if let Some(body) = catalog.minor_planets().iter().find(|body| body.id() == id) {
        let radius = [body.axis_a(), body.axis_b(), body.axis_c()]
            .into_iter()
            .flatten()
            .reduce(|a, b| if a.value() > b.value() { a } else { b });
        let mut details = new_details(
            id,
            Some(body.mass()),
            radius,
            body.rotation_period().map(|v| v.value()),
            body.orbital_elements(),
            body.orbital_parent(),
            catalog,
        );
        if let (Some(a), Some(b), Some(c)) = (body.axis_a(), body.axis_b(), body.axis_c()) {
            let volume = (4.0 / 3.0) * PI * a.value() * b.value() * c.value();
            details.density_kg_m3 = valid(Some(body.mass().value() / volume));
            details.reference_radius_gravity = true;
        }
        celestial_body_facts::minor(&mut details.facts, &mut details.text_facts, body);
        append_system(&mut details, pool, body.star_system_id()).await?;
        return Ok(details);
    }
    if let Some(body) = catalog.barycenters().iter().find(|body| body.id() == id) {
        let mut details = new_details(
            id,
            None,
            None,
            None,
            body.external_orbital_elements(),
            body.orbital_parent(),
            catalog,
        );
        let inner = body.internal_orbital_elements();
        celestial_body_facts::push(
            &mut details.facts,
            "barycenter",
            "internal_semi_major",
            Some(inner.semi_major_axis().value()),
        );
        celestial_body_facts::push(
            &mut details.facts,
            "barycenter",
            "internal_eccentricity",
            Some(inner.eccentricity()),
        );
        celestial_body_facts::push(
            &mut details.facts,
            "barycenter",
            "internal_inclination",
            Some(inner.inclination().value().to_degrees()),
        );
        celestial_body_facts::orbit(&mut details.facts, body.external_orbital_elements());
        append_system(&mut details, pool, body.star_system_id()).await?;
        for (key, member) in [
            ("primary_member", body.member_primary()),
            ("secondary_member", body.member_secondary()),
        ] {
            let id = member.id();
            let name = catalog
                .stars()
                .iter()
                .find(|s| s.id() == id)
                .map(|s| s.name())
                .or_else(|| {
                    catalog
                        .planets()
                        .iter()
                        .find(|s| s.id() == id)
                        .map(|s| s.name())
                })
                .or_else(|| {
                    catalog
                        .barycenters()
                        .iter()
                        .find(|s| s.id() == id)
                        .map(|s| s.name())
                });
            if let Some(name) = name {
                details.text_facts.push(BodyTextFact {
                    section: "barycenter".into(),
                    key: key.into(),
                    value: name.into(),
                });
            }
        }
        return Ok(details);
    }
    Err(RocketError::Generic(
        "selected celestial body is not in the catalog".into(),
    ))
}
