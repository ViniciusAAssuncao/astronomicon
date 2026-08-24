use crate::domain::{Barycenter, BarycenterMember, MinorPlanet, OrbitalParent, Planet, Star};
use crate::error::{DomainError, DomainResult};
use crate::units::Mass;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn calculate_effective_mass_inner(
    member: &BarycenterMember,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    visited: &mut HashSet<Uuid>,
) -> DomainResult<Mass> {
    match member {
        BarycenterMember::Star(id) => {
            let star = stars
                .get(id)
                .copied()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "member_primary/secondary".to_string(),
                    reason: format!("star '{}' not found", id),
                })?;
            Ok(star.mass())
        }
        BarycenterMember::Planet(id) => {
            let planet = planets
                .get(id)
                .copied()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "member_primary/secondary".to_string(),
                    reason: format!("planet '{}' not found", id),
                })?;
            Ok(planet.mass())
        }
        BarycenterMember::Barycenter(id) => {
            if !visited.insert(*id) {
                return Err(DomainError::InvalidInvariant {
                    field: "barycenter".to_string(),
                    reason: format!("circular reference detected in barycenter '{}'", id),
                });
            }

            let bary =
                barycenters
                    .get(id)
                    .copied()
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "member_primary/secondary".to_string(),
                        reason: format!("barycenter '{}' not found", id),
                    })?;

            let m_pri = calculate_effective_mass_inner(
                &bary.member_primary(),
                stars,
                planets,
                barycenters,
                visited,
            )?;
            let m_sec = calculate_effective_mass_inner(
                &bary.member_secondary(),
                stars,
                planets,
                barycenters,
                visited,
            )?;

            visited.remove(id);
            Ok(Mass::new(m_pri.value() + m_sec.value()))
        }
    }
}

pub fn calculate_effective_mass(
    member: &BarycenterMember,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
) -> DomainResult<Mass> {
    let mut visited = HashSet::new();
    calculate_effective_mass_inner(member, stars, planets, barycenters, &mut visited)
}

pub fn calculate_parent_effective_mass(
    parent: &OrbitalParent,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
) -> DomainResult<Mass> {
    match parent {
        OrbitalParent::Fixed => Ok(Mass::new(0.0)),
        OrbitalParent::Star(id) => {
            let star = stars
                .get(id)
                .copied()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: format!("parent star '{}' not found", id),
                })?;
            Ok(star.mass())
        }
        OrbitalParent::Planet(id) => {
            let planet = planets
                .get(id)
                .copied()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "orbital_parent".to_string(),
                    reason: format!("parent planet '{}' not found", id),
                })?;
            Ok(planet.mass())
        }
        OrbitalParent::Barycenter(id) => calculate_effective_mass(
            &BarycenterMember::Barycenter(*id),
            stars,
            planets,
            barycenters,
        ),
        OrbitalParent::MinorPlanet(id) => {
            let minor_planet =
                minor_planets
                    .get(id)
                    .copied()
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "orbital_parent".to_string(),
                        reason: format!("parent minor planet '{}' not found", id),
                    })?;
            Ok(minor_planet.mass())
        }
    }
}
