use astronomicon_core::domain::{
    Barycenter, BarycenterMember, MinorPlanet, OrbitalParent, Planet, Star,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn expand_barycenter_member<'a>(
    member: &BarycenterMember,
    stars: &HashMap<Uuid, &'a Star>,
    barycenters: &HashMap<Uuid, &'a Barycenter>,
    visited: &mut HashSet<Uuid>,
    collected: &mut Vec<&'a Star>,
) {
    match member {
        BarycenterMember::Star(id) => {
            if let Some(star) = stars.get(id) {
                if visited.insert(*id) {
                    collected.push(star);
                }
            }
        }
        BarycenterMember::Barycenter(id) => {
            if visited.insert(*id) {
                if let Some(bary) = barycenters.get(id) {
                    expand_barycenter_member(
                        &bary.member_primary(),
                        stars,
                        barycenters,
                        visited,
                        collected,
                    );
                    expand_barycenter_member(
                        &bary.member_secondary(),
                        stars,
                        barycenters,
                        visited,
                        collected,
                    );
                }
            }
        }
        BarycenterMember::Planet(_) => {}
    }
}

pub fn collect_gravitationally_linked_stars<'a>(
    planet: &Planet,
    stars: &HashMap<Uuid, &'a Star>,
    planets: &HashMap<Uuid, &'a Planet>,
    barycenters: &HashMap<Uuid, &'a Barycenter>,
    minor_planets: &HashMap<Uuid, &'a MinorPlanet>,
) -> Vec<&'a Star> {
    let mut visited_entities = HashSet::new();
    let mut visited_stars = HashSet::new();
    let mut result = Vec::new();

    let mut current_parent = planet.orbital_parent();

    while current_parent != OrbitalParent::Fixed {
        match current_parent {
            OrbitalParent::Star(star_id) => {
                if !visited_entities.insert(star_id) {
                    break;
                }
                if let Some(star) = stars.get(&star_id) {
                    if visited_stars.insert(star_id) {
                        result.push(*star);
                    }
                    for bary in barycenters.values() {
                        if bary.member_primary().id() == star_id
                            || bary.member_secondary().id() == star_id
                        {
                            expand_barycenter_member(
                                &bary.member_primary(),
                                stars,
                                barycenters,
                                &mut visited_stars,
                                &mut result,
                            );
                            expand_barycenter_member(
                                &bary.member_secondary(),
                                stars,
                                barycenters,
                                &mut visited_stars,
                                &mut result,
                            );
                        }
                    }
                    current_parent = star.orbital_parent();
                } else {
                    break;
                }
            }
            OrbitalParent::Barycenter(bary_id) => {
                if !visited_entities.insert(bary_id) {
                    break;
                }
                if let Some(bary) = barycenters.get(&bary_id) {
                    expand_barycenter_member(
                        &bary.member_primary(),
                        stars,
                        barycenters,
                        &mut visited_stars,
                        &mut result,
                    );
                    expand_barycenter_member(
                        &bary.member_secondary(),
                        stars,
                        barycenters,
                        &mut visited_stars,
                        &mut result,
                    );
                    current_parent = bary.orbital_parent();
                } else {
                    break;
                }
            }
            OrbitalParent::Planet(p_id) => {
                if !visited_entities.insert(p_id) {
                    break;
                }
                if let Some(p) = planets.get(&p_id) {
                    current_parent = p.orbital_parent();
                } else {
                    break;
                }
            }
            OrbitalParent::MinorPlanet(mp_id) => {
                if !visited_entities.insert(mp_id) {
                    break;
                }
                if let Some(mp) = minor_planets.get(&mp_id) {
                    current_parent = mp.orbital_parent();
                } else {
                    break;
                }
            }
            OrbitalParent::Fixed => break,
        }
    }

    result
}