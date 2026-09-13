use crate::domain::{
    Barycenter, BarycenterMember, MinorPlanet, OrbitalParent, Planet, Star,
};
use crate::error::{DomainError, DomainResult};
use crate::math::gravity::hierarchy_mass::{
    calculate_effective_mass, calculate_parent_effective_mass,
};
use crate::math::gravity::point_mass::combined_gravitational_parameter;
use crate::math::kepler::orbital_position;
use crate::units::{Duration, Mass, Position};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

enum EntityKind<'a> {
    Star(&'a Star),
    Planet(&'a Planet),
    Barycenter(&'a Barycenter),
    MinorPlanet(&'a MinorPlanet),
}

fn find_entity<'a>(
    id: Uuid,
    stars: &'a HashMap<Uuid, &Star>,
    planets: &'a HashMap<Uuid, &Planet>,
    barycenters: &'a HashMap<Uuid, &Barycenter>,
    minor_planets: &'a HashMap<Uuid, &MinorPlanet>,
) -> DomainResult<EntityKind<'a>> {
    if let Some(star) = stars.get(&id) {
        return Ok(EntityKind::Star(star));
    }
    if let Some(planet) = planets.get(&id) {
        return Ok(EntityKind::Planet(planet));
    }
    if let Some(bary) = barycenters.get(&id) {
        return Ok(EntityKind::Barycenter(bary));
    }
    if let Some(minor) = minor_planets.get(&id) {
        return Ok(EntityKind::MinorPlanet(minor));
    }
    Err(DomainError::InvalidInvariant {
        field: "entity".to_string(),
        reason: format!("entity '{}' not found", id),
    })
}

pub fn barycentric_member_offset(
    relative_position: Position,
    primary_mass: Mass,
    secondary_mass: Mass,
    is_secondary: bool,
) -> Position {
    let m1 = primary_mass.value();
    let m2 = secondary_mass.value();

    if !m1.is_finite() || !m2.is_finite() || m1 < 0.0 || m2 < 0.0 {
        return Position::zero();
    }

    let total = m1 + m2;
    if total <= 0.0 {
        return Position::zero();
    }

    let raw = relative_position.raw();
    if !raw.0.is_finite() || !raw.1.is_finite() || !raw.2.is_finite() {
        return Position::zero();
    }

    if is_secondary {
        relative_position * (m1 / total)
    } else {
        relative_position * (-m2 / total)
    }
}

fn calculate_parent_absolute_position_inner(
    parent: &OrbitalParent,
    time_since_epoch: Duration,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
    visited: &mut HashSet<Uuid>,
) -> DomainResult<Position> {
    match parent {
        OrbitalParent::Fixed => Ok(Position::zero()),
        OrbitalParent::Star(id) => {
            if !stars.contains_key(id) {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: format!("parent star '{}' not found", id),
                });
            }
            calculate_absolute_position_inner(
                *id,
                time_since_epoch,
                stars,
                planets,
                barycenters,
                minor_planets,
                visited,
            )
        }
        OrbitalParent::Planet(id) => {
            if !planets.contains_key(id) {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: format!("parent planet '{}' not found", id),
                });
            }
            calculate_absolute_position_inner(
                *id,
                time_since_epoch,
                stars,
                planets,
                barycenters,
                minor_planets,
                visited,
            )
        }
        OrbitalParent::Barycenter(id) => {
            if !barycenters.contains_key(id) {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: format!("parent barycenter '{}' not found", id),
                });
            }
            calculate_absolute_position_inner(
                *id,
                time_since_epoch,
                stars,
                planets,
                barycenters,
                minor_planets,
                visited,
            )
        }
        OrbitalParent::MinorPlanet(id) => {
            if !minor_planets.contains_key(id) {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: format!("parent minor planet '{}' not found", id),
                });
            }
            calculate_absolute_position_inner(
                *id,
                time_since_epoch,
                stars,
                planets,
                barycenters,
                minor_planets,
                visited,
            )
        }
    }
}

fn calculate_absolute_position_dispatch(
    id: Uuid,
    time_since_epoch: Duration,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
    visited: &mut HashSet<Uuid>,
) -> DomainResult<Position> {
    let entity = find_entity(id, stars, planets, barycenters, minor_planets)?;

    let parent = match entity {
        EntityKind::Star(s) => s.orbital_parent(),
        EntityKind::Planet(p) => p.orbital_parent(),
        EntityKind::Barycenter(b) => b.orbital_parent(),
        EntityKind::MinorPlanet(m) => m.orbital_parent(),
    };

    let elements = match entity {
        EntityKind::Star(s) => s.orbital_elements(),
        EntityKind::Planet(p) => p.orbital_elements(),
        EntityKind::Barycenter(b) => b.external_orbital_elements(),
        EntityKind::MinorPlanet(m) => m.orbital_elements(),
    };

    let mass = match entity {
        EntityKind::Star(s) => s.mass(),
        EntityKind::Planet(p) => p.mass(),
        EntityKind::MinorPlanet(m) => m.mass(),
        EntityKind::Barycenter(_) => calculate_effective_mass(
            &BarycenterMember::Barycenter(id),
            stars,
            planets,
            barycenters,
        )?,
    };

    if parent == OrbitalParent::Fixed {
        return Ok(Position::zero());
    }

    if let OrbitalParent::Barycenter(bary_id) = parent {
        let bary = barycenters
            .get(&bary_id)
            .copied()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_parent".to_string(),
                reason: format!("parent barycenter '{}' not found", bary_id),
            })?;

        let is_primary = bary.member_primary().id() == id;
        let is_secondary = bary.member_secondary().id() == id;

        if is_primary || is_secondary {
            let m_pri = calculate_effective_mass(
                &bary.member_primary(),
                stars,
                planets,
                barycenters,
            )?;
            let m_sec = calculate_effective_mass(
                &bary.member_secondary(),
                stars,
                planets,
                barycenters,
            )?;
            let mu = combined_gravitational_parameter(m_pri, m_sec);
            let r_rel = orbital_position(&bary.internal_orbital_elements(), mu, time_since_epoch)?;
            let offset = barycentric_member_offset(r_rel, m_pri, m_sec, is_secondary);
            let parent_pos = calculate_absolute_position_inner(
                bary_id,
                time_since_epoch,
                stars,
                planets,
                barycenters,
                minor_planets,
                visited,
            )?;
            return Ok(parent_pos + offset);
        }
    }

    let parent_pos = calculate_parent_absolute_position_inner(
        &parent,
        time_since_epoch,
        stars,
        planets,
        barycenters,
        minor_planets,
        visited,
    )?;

    let parent_mass = calculate_parent_effective_mass(
        &parent,
        stars,
        planets,
        barycenters,
        minor_planets,
    )?;

    let mu = combined_gravitational_parameter(parent_mass, mass);

    let orb_elements = elements.ok_or_else(|| DomainError::InvalidInvariant {
        field: "orbital_elements".to_string(),
        reason: format!("entity '{}' has non-fixed parent but no orbital elements", id),
    })?;

    let r_rel = orbital_position(&orb_elements, mu, time_since_epoch)?;
    Ok(parent_pos + r_rel)
}

fn calculate_absolute_position_inner(
    id: Uuid,
    time_since_epoch: Duration,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
    visited: &mut HashSet<Uuid>,
) -> DomainResult<Position> {
    if !visited.insert(id) {
        return Err(DomainError::InvalidInvariant {
            field: "orbital_parent".to_string(),
            reason: format!("circular reference detected for entity '{}'", id),
        });
    }

    let result = calculate_absolute_position_dispatch(
        id,
        time_since_epoch,
        stars,
        planets,
        barycenters,
        minor_planets,
        visited,
    );

    visited.remove(&id);
    result
}

pub fn calculate_absolute_position(
    id: Uuid,
    time_since_epoch: Duration,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
) -> DomainResult<Position> {
    if !time_since_epoch.value().is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "time_since_epoch".to_string(),
            reason: "value must be finite".to_string(),
        });
    }
    let mut visited = HashSet::new();
    calculate_absolute_position_inner(
        id,
        time_since_epoch,
        stars,
        planets,
        barycenters,
        minor_planets,
        &mut visited,
    )
}

pub fn calculate_parent_absolute_position(
    parent: &OrbitalParent,
    time_since_epoch: Duration,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
) -> DomainResult<Position> {
    if !time_since_epoch.value().is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "time_since_epoch".to_string(),
            reason: "value must be finite".to_string(),
        });
    }
    let mut visited = HashSet::new();
    calculate_parent_absolute_position_inner(
        parent,
        time_since_epoch,
        stars,
        planets,
        barycenters,
        minor_planets,
        &mut visited,
    )
}

pub fn calculate_member_absolute_position(
    member: &BarycenterMember,
    time_since_epoch: Duration,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
) -> DomainResult<Position> {
    calculate_absolute_position(
        member.id(),
        time_since_epoch,
        stars,
        planets,
        barycenters,
        minor_planets,
    )
}
