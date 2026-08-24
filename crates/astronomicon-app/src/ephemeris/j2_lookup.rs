use astronomicon_core::domain::{MinorPlanet, OrbitalParent, Planet, Star};
use astronomicon_core::math::minor_planet::equivalent_spherical_radius;
use astronomicon_core::units::Length;
use std::collections::HashMap;
use uuid::Uuid;

pub fn get_parent_j2_and_radius(
    parent: &OrbitalParent,
    star_map: &HashMap<Uuid, &Star>,
    planet_map: &HashMap<Uuid, &Planet>,
    minor_planet_map: &HashMap<Uuid, &MinorPlanet>,
) -> (Option<f64>, Option<Length>) {
    match parent {
        OrbitalParent::Star(pid) => {
            let p = star_map.get(pid).copied();
            (
                p.and_then(|s| s.oblateness_j2()),
                p.and_then(|s| s.radius()),
            )
        }
        OrbitalParent::Planet(pid) => {
            let p = planet_map.get(pid).copied();
            (
                p.and_then(|pl| pl.oblateness_j2()),
                p.and_then(|pl| pl.equatorial_radius()),
            )
        }
        OrbitalParent::MinorPlanet(pid) => {
            let p = minor_planet_map.get(pid).copied();
            let rad = p.and_then(|mp| match (mp.axis_a(), mp.axis_b(), mp.axis_c()) {
                (Some(a), Some(b), Some(c)) => Some(equivalent_spherical_radius(a, b, c)),
                _ => None,
            });
            (None, rad)
        }
        OrbitalParent::Barycenter(_) | OrbitalParent::Fixed => (None, None),
    }
}
