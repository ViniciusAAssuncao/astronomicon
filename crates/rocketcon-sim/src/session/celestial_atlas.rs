use super::celestial_orbit_curve::catalog_orbit_points;
use crate::{RocketError, RocketResult};
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::domain::OrbitalElements;
use astronomicon_core::domain::{OrbitalParent, PlanetKind};
use astronomicon_core::units::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct CatalogBody {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub kind: u8,
    pub radius_m: Option<f64>,
    pub mass_kg: Option<f64>,
    pub relative_position_m: [f64; 3],
    pub orbit_points_m: Vec<[f64; 3]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CelestialAtlas {
    pub reference_body_id: Uuid,
    pub epoch_seconds: f64,
    pub bodies: Vec<CatalogBody>,
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

pub fn catalog_at(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    reference: Uuid,
) -> RocketResult<CelestialAtlas> {
    let positions = catalog
        .positions(epoch)
        .map_err(|error| RocketError::Generic(error.to_string()))?;
    let center = positions
        .get(&reference)
        .ok_or_else(|| RocketError::Generic("atlas reference center missing".into()))?
        .raw();
    let mut bodies = Vec::with_capacity(
        catalog.stars().len()
            + catalog.planets().len()
            + catalog.minor_planets().len()
            + catalog.barycenters().len(),
    );
    let mut add = |id: Uuid,
                   parent_id: Option<Uuid>,
                   name: &str,
                   kind: u8,
                   radius_m: Option<f64>,
                   mass_kg: Option<f64>,
                   elements: Option<OrbitalElements>| {
        if let Some(position) = positions.get(&id) {
            let point = position.raw() - center;
            let coordinates = [point.0, point.1, point.2];
            if coordinates.iter().all(|v| v.is_finite()) {
                let orbit_points_m = parent_id
                    .and_then(|parent| positions.get(&parent))
                    .zip(elements)
                    .map_or_else(Vec::new, |(parent, orbit)| {
                        let p = parent.raw();
                        let origin = center;
                        catalog_orbit_points(orbit, [p.0, p.1, p.2], [origin.0, origin.1, origin.2])
                    });
                bodies.push(CatalogBody {
                    id,
                    parent_id,
                    name: name.to_owned(),
                    kind,
                    radius_m: radius_m.filter(|r| r.is_finite() && *r > 0.0),
                    mass_kg: mass_kg.filter(|m| m.is_finite() && *m > 0.0),
                    relative_position_m: coordinates,
                    orbit_points_m,
                });
            }
        }
    };
    for star in catalog.stars() {
        add(
            star.id(),
            parent_id(star.orbital_parent()),
            star.name(),
            0,
            star.radius().map(|r| r.value()),
            Some(star.mass().value()),
            star.orbital_elements(),
        );
    }
    for planet in catalog.planets() {
        let parent = planet.orbital_parent();
        let kind = if planet.kind() == PlanetKind::DwarfPlanet {
            3
        } else if matches!(parent, OrbitalParent::Planet(_)) {
            2
        } else {
            1
        };
        add(
            planet.id(),
            parent_id(parent),
            planet.name(),
            kind,
            planet
                .equatorial_radius()
                .or_else(|| planet.polar_radius())
                .map(|r| r.value()),
            Some(planet.mass().value()),
            planet.orbital_elements(),
        );
    }
    for minor in catalog.minor_planets() {
        let radius = [minor.axis_a(), minor.axis_b(), minor.axis_c()]
            .into_iter()
            .flatten()
            .map(|r| r.value())
            .filter(|r| r.is_finite() && *r > 0.0)
            .reduce(f64::max);
        add(
            minor.id(),
            parent_id(minor.orbital_parent()),
            minor.name(),
            4,
            radius,
            Some(minor.mass().value()),
            minor.orbital_elements(),
        );
    }
    for barycenter in catalog.barycenters() {
        add(
            barycenter.id(),
            parent_id(barycenter.orbital_parent()),
            barycenter.name(),
            5,
            None,
            None,
            barycenter.external_orbital_elements(),
        );
    }
    bodies.sort_unstable_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    Ok(CelestialAtlas {
        reference_body_id: reference,
        epoch_seconds: epoch.value(),
        bodies,
    })
}
