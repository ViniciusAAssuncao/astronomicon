use crate::ephemeris::barycenter::split_barycenter_positions;
use crate::ephemeris::j2_lookup::get_parent_j2_and_radius;
use astronomicon_core::domain::{
    Barycenter, BarycenterMember, MinorPlanet, OrbitalElements, OrbitalParent, Planet, Star,
};
use astronomicon_core::error::{DomainError, DomainResult};
use astronomicon_core::math::gravity::{
    calculate_effective_mass, calculate_parent_effective_mass, combined_gravitational_parameter,
};
use astronomicon_core::math::kepler::orbital_position_secular;
use astronomicon_core::math::perturbation::resolve_secular_precession;
use astronomicon_core::units::{Duration, Mass, Position};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn resolve_relative_position_from_parent(
    parent_pos: Position,
    node_mass: Mass,
    parent: &OrbitalParent,
    elements: &OrbitalElements,
    star_map: &HashMap<Uuid, &Star>,
    planet_map: &HashMap<Uuid, &Planet>,
    barycenter_map: &HashMap<Uuid, &Barycenter>,
    minor_planet_map: &HashMap<Uuid, &MinorPlanet>,
    time_since_epoch: Duration,
) -> DomainResult<Position> {
    let m_parent = calculate_parent_effective_mass(
        parent,
        star_map,
        planet_map,
        barycenter_map,
        minor_planet_map,
    )?;
    let mu = combined_gravitational_parameter(node_mass, m_parent);
    let (parent_j2, parent_radius) =
        get_parent_j2_and_radius(parent, star_map, planet_map, minor_planet_map);
    let secular_rates = resolve_secular_precession(elements, mu, parent_j2, parent_radius);
    let rel_pos = orbital_position_secular(elements, mu, &secular_rates, time_since_epoch)?;
    Ok(parent_pos + rel_pos)
}

pub fn resolve_node(
    id: Uuid,
    member_of: &HashMap<Uuid, Uuid>,
    star_map: &HashMap<Uuid, &Star>,
    planet_map: &HashMap<Uuid, &Planet>,
    barycenter_map: &HashMap<Uuid, &Barycenter>,
    minor_planet_map: &HashMap<Uuid, &MinorPlanet>,
    memo: &mut HashMap<Uuid, Position>,
    visiting: &mut HashSet<Uuid>,
    time_since_epoch: Duration,
) -> DomainResult<Position> {
    if let Some(&pos) = memo.get(&id) {
        return Ok(pos);
    }

    if !visiting.insert(id) {
        return Err(DomainError::InvalidInvariant {
            field: "orbital_hierarchy".to_string(),
            reason: format!("circular dependency detected for entity '{}'", id),
        });
    }

    if let Some(&parent_bary_id) = member_of.get(&id) {
        let bary_pos = resolve_node(
            parent_bary_id,
            member_of,
            star_map,
            planet_map,
            barycenter_map,
            minor_planet_map,
            memo,
            visiting,
            time_since_epoch,
        )?;

        split_barycenter_positions(
            parent_bary_id,
            bary_pos,
            barycenter_map,
            star_map,
            planet_map,
            minor_planet_map,
            memo,
            time_since_epoch,
        )?;

        visiting.remove(&id);

        return memo.get(&id).copied().ok_or_else(|| {
            DomainError::InvalidInvariant {
                field: "barycenter_member".to_string(),
                reason: format!(
                    "position for member '{}' not resolved by barycenter '{}'",
                    id, parent_bary_id
                ),
            }
        });
    }

    let pos = if let Some(star) = star_map.get(&id).copied() {
        match star.orbital_parent() {
            OrbitalParent::Fixed => Position::zero(),
            parent => {
                let parent_id = match parent {
                    OrbitalParent::Star(pid)
                    | OrbitalParent::Planet(pid)
                    | OrbitalParent::Barycenter(pid)
                    | OrbitalParent::MinorPlanet(pid) => pid,
                    OrbitalParent::Fixed => unreachable!(),
                };
                let parent_pos = resolve_node(
                    parent_id,
                    member_of,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    memo,
                    visiting,
                    time_since_epoch,
                )?;
                let elements = star.orbital_elements().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "orbital_elements".to_string(),
                        reason: format!("star '{}' has orbital parent but no orbital elements", id),
                    }
                })?;
                resolve_relative_position_from_parent(
                    parent_pos,
                    star.mass(),
                    &parent,
                    &elements,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    time_since_epoch,
                )?
            }
        }
    } else if let Some(planet) = planet_map.get(&id).copied() {
        match planet.orbital_parent() {
            OrbitalParent::Fixed => Position::zero(),
            parent => {
                let parent_id = match parent {
                    OrbitalParent::Star(pid)
                    | OrbitalParent::Planet(pid)
                    | OrbitalParent::Barycenter(pid)
                    | OrbitalParent::MinorPlanet(pid) => pid,
                    OrbitalParent::Fixed => unreachable!(),
                };
                let parent_pos = resolve_node(
                    parent_id,
                    member_of,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    memo,
                    visiting,
                    time_since_epoch,
                )?;
                let elements = planet.orbital_elements().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "orbital_elements".to_string(),
                        reason: format!("planet '{}' has orbital parent but no orbital elements", id),
                    }
                })?;
                resolve_relative_position_from_parent(
                    parent_pos,
                    planet.mass(),
                    &parent,
                    &elements,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    time_since_epoch,
                )?
            }
        }
    } else if let Some(minor_planet) = minor_planet_map.get(&id).copied() {
        match minor_planet.orbital_parent() {
            OrbitalParent::Fixed => Position::zero(),
            parent => {
                let parent_id = match parent {
                    OrbitalParent::Star(pid)
                    | OrbitalParent::Planet(pid)
                    | OrbitalParent::Barycenter(pid)
                    | OrbitalParent::MinorPlanet(pid) => pid,
                    OrbitalParent::Fixed => unreachable!(),
                };
                let parent_pos = resolve_node(
                    parent_id,
                    member_of,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    memo,
                    visiting,
                    time_since_epoch,
                )?;
                let elements = minor_planet.orbital_elements().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "orbital_elements".to_string(),
                        reason: format!(
                            "minor planet '{}' has orbital parent but no orbital elements",
                            id
                        ),
                    }
                })?;
                resolve_relative_position_from_parent(
                    parent_pos,
                    minor_planet.mass(),
                    &parent,
                    &elements,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    time_since_epoch,
                )?
            }
        }
    } else if let Some(bary) = barycenter_map.get(&id).copied() {
        let bary_pos = match bary.orbital_parent() {
            OrbitalParent::Fixed => Position::zero(),
            parent => {
                let parent_id = match parent {
                    OrbitalParent::Star(pid)
                    | OrbitalParent::Planet(pid)
                    | OrbitalParent::Barycenter(pid)
                    | OrbitalParent::MinorPlanet(pid) => pid,
                    OrbitalParent::Fixed => unreachable!(),
                };
                let parent_pos = resolve_node(
                    parent_id,
                    member_of,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    memo,
                    visiting,
                    time_since_epoch,
                )?;
                let elements = bary.external_orbital_elements().ok_or_else(|| {
                    DomainError::InvalidInvariant {
                        field: "external_orbital_elements".to_string(),
                        reason: format!(
                            "barycenter '{}' has orbital parent but no external orbital elements",
                            id
                        ),
                    }
                })?;
                let m_node = calculate_effective_mass(
                    &BarycenterMember::Barycenter(id),
                    star_map,
                    planet_map,
                    barycenter_map,
                )?;
                resolve_relative_position_from_parent(
                    parent_pos,
                    m_node,
                    &parent,
                    &elements,
                    star_map,
                    planet_map,
                    barycenter_map,
                    minor_planet_map,
                    time_since_epoch,
                )?
            }
        };

        memo.insert(id, bary_pos);

        split_barycenter_positions(
            id,
            bary_pos,
            barycenter_map,
            star_map,
            planet_map,
            minor_planet_map,
            memo,
            time_since_epoch,
        )?;

        bary_pos
    } else {
        return Err(DomainError::InvalidInvariant {
            field: "entity_id".to_string(),
            reason: format!("entity '{}' not found in system hierarchy", id),
        });
    };

    memo.insert(id, pos);
    visiting.remove(&id);
    Ok(pos)
}
