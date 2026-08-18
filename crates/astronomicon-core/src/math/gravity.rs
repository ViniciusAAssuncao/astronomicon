use crate::domain::{Barycenter, BarycenterMember, OrbitalParent, Planet, Star};
use crate::error::{DomainError, DomainResult};
use crate::units::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{
    Acceleration, AccelerationVector, GravitationalParameter, Length, Mass, Position, Vector3,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub fn gravitational_parameter(mass: Mass) -> GravitationalParameter {
    GravitationalParameter::new(GRAVITATIONAL_CONSTANT * mass.value())
}

pub fn combined_gravitational_parameter(mass_a: Mass, mass_b: Mass) -> GravitationalParameter {
    gravitational_parameter(mass_a + mass_b)
}

pub fn surface_gravity(mu: GravitationalParameter, radius: Length) -> Acceleration {
    if radius.value() <= 0.0 {
        Acceleration::new(0.0)
    } else {
        Acceleration::new(mu.value() / (radius.value() * radius.value()))
    }
}

pub fn gravitational_acceleration_at_altitude(
    mu: GravitationalParameter,
    equatorial_radius: Length,
    altitude: Length,
) -> Acceleration {
    let r = equatorial_radius.value() + altitude.value();
    if r <= 0.0 {
        Acceleration::new(0.0)
    } else {
        Acceleration::new(mu.value() / (r * r))
    }
}

pub fn gravitational_acceleration_at(
    point: Position,
    sources: &[(Position, Mass)],
) -> AccelerationVector {
    let mut total_acc = Vector3::zero();
    let p = point.raw();

    for (pos, mass) in sources {
        let diff = pos.raw() - p;
        let dist = diff.magnitude();
        if dist > 1e-6 && mass.value() > 0.0 {
            let factor = GRAVITATIONAL_CONSTANT * mass.value() / (dist * dist * dist);
            total_acc = total_acc + diff * factor;
        }
    }

    AccelerationVector::from_raw(total_acc)
}

fn calculate_effective_mass_inner(
    member: &BarycenterMember,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    visited: &mut HashSet<Uuid>,
) -> DomainResult<Mass> {
    match member {
        BarycenterMember::Star(id) => {
            let star = stars.get(id).copied().ok_or_else(|| DomainError::InvalidInvariant {
                field: "member_primary/secondary".to_string(),
                reason: format!("star '{}' not found", id),
            })?;
            Ok(star.mass())
        }
        BarycenterMember::Planet(id) => {
            let planet = planets.get(id).copied().ok_or_else(|| DomainError::InvalidInvariant {
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

            let bary = barycenters.get(id).copied().ok_or_else(|| DomainError::InvalidInvariant {
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
) -> DomainResult<Mass> {
    match parent {
        OrbitalParent::Fixed => Ok(Mass::new(0.0)),
        OrbitalParent::Star(id) => {
            let star = stars.get(id).copied().ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_parent".to_string(),
                reason: format!("parent star '{}' not found", id),
            })?;
            Ok(star.mass())
        }
        OrbitalParent::Planet(id) => {
            let planet = planets.get(id).copied().ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_parent".to_string(),
                reason: format!("parent planet '{}' not found", id),
            })?;
            Ok(planet.mass())
        }
        OrbitalParent::Barycenter(id) => {
            calculate_effective_mass(
                &BarycenterMember::Barycenter(*id),
                stars,
                planets,
                barycenters,
            )
        }
    }
}