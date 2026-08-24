use crate::error::AppResult;
use crate::hierarchy::fetch_system_hierarchy;
use astronomicon_core::domain::{
    Barycenter, BarycenterMember, MinorPlanet, OrbitalElements, Planet, Star,
};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{
    calculate_effective_mass, calculate_parent_effective_mass, combined_gravitational_parameter,
};
use astronomicon_core::math::kepler::{mean_motion, orbital_period};
use astronomicon_core::units::{AngularVelocity, Duration};
use astronomicon_db::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct BodyOrbitInfo {
    pub elements: OrbitalElements,
    pub mean_motion: AngularVelocity,
    pub period: Duration,
}

pub struct SystemHierarchyContext {
    pub star_map: HashMap<Uuid, Star>,
    pub planet_map: HashMap<Uuid, Planet>,
    pub barycenter_map: HashMap<Uuid, Barycenter>,
    pub minor_planet_map: HashMap<Uuid, MinorPlanet>,
}

impl SystemHierarchyContext {
    pub fn get_body_orbit_info(&self, body_id: &Uuid) -> AppResult<BodyOrbitInfo> {
        let star_ref_map: HashMap<Uuid, &Star> =
            self.star_map.iter().map(|(k, v)| (*k, v)).collect();
        let planet_ref_map: HashMap<Uuid, &Planet> =
            self.planet_map.iter().map(|(k, v)| (*k, v)).collect();
        let bary_ref_map: HashMap<Uuid, &Barycenter> =
            self.barycenter_map.iter().map(|(k, v)| (*k, v)).collect();
        let minor_planet_ref_map: HashMap<Uuid, &MinorPlanet> =
            self.minor_planet_map.iter().map(|(k, v)| (*k, v)).collect();

        if let Some(planet) = self.planet_map.get(body_id) {
            let elements =
                planet
                    .orbital_elements()
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "orbital_elements".to_string(),
                        reason: format!("planet '{}' has no orbital elements", body_id),
                    })?;
            let parent_mass = calculate_parent_effective_mass(
                &planet.orbital_parent(),
                &star_ref_map,
                &planet_ref_map,
                &bary_ref_map,
                &minor_planet_ref_map,
            )?;
            let mu = combined_gravitational_parameter(planet.mass(), parent_mass);
            let n = mean_motion(elements.semi_major_axis(), mu);
            let period = orbital_period(elements.semi_major_axis(), mu).ok_or_else(|| {
                DomainError::InvalidInvariant {
                    field: "orbital_period".to_string(),
                    reason: format!("invalid orbital period for planet '{}'", body_id),
                }
            })?;
            Ok(BodyOrbitInfo {
                elements,
                mean_motion: n,
                period,
            })
        } else if let Some(star) = self.star_map.get(body_id) {
            let elements =
                star.orbital_elements()
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "orbital_elements".to_string(),
                        reason: format!("star '{}' has no orbital elements", body_id),
                    })?;
            let parent_mass = calculate_parent_effective_mass(
                &star.orbital_parent(),
                &star_ref_map,
                &planet_ref_map,
                &bary_ref_map,
                &minor_planet_ref_map,
            )?;
            let mu = combined_gravitational_parameter(star.mass(), parent_mass);
            let n = mean_motion(elements.semi_major_axis(), mu);
            let period = orbital_period(elements.semi_major_axis(), mu).ok_or_else(|| {
                DomainError::InvalidInvariant {
                    field: "orbital_period".to_string(),
                    reason: format!("invalid orbital period for star '{}'", body_id),
                }
            })?;
            Ok(BodyOrbitInfo {
                elements,
                mean_motion: n,
                period,
            })
        } else if let Some(bary) = self.barycenter_map.get(body_id) {
            let elements =
                bary.external_orbital_elements()
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "external_orbital_elements".to_string(),
                        reason: format!(
                            "barycenter '{}' has no external orbital elements",
                            body_id
                        ),
                    })?;
            let inner_mass = calculate_effective_mass(
                &BarycenterMember::Barycenter(*body_id),
                &star_ref_map,
                &planet_ref_map,
                &bary_ref_map,
            )?;
            let parent_mass = calculate_parent_effective_mass(
                &bary.orbital_parent(),
                &star_ref_map,
                &planet_ref_map,
                &bary_ref_map,
                &minor_planet_ref_map,
            )?;
            let mu = combined_gravitational_parameter(inner_mass, parent_mass);
            let n = mean_motion(elements.semi_major_axis(), mu);
            let period = orbital_period(elements.semi_major_axis(), mu).ok_or_else(|| {
                DomainError::InvalidInvariant {
                    field: "orbital_period".to_string(),
                    reason: format!("invalid orbital period for barycenter '{}'", body_id),
                }
            })?;
            Ok(BodyOrbitInfo {
                elements,
                mean_motion: n,
                period,
            })
        } else if let Some(mp) = self.minor_planet_map.get(body_id) {
            let elements = mp
                .orbital_elements()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "orbital_elements".to_string(),
                    reason: format!("minor planet '{}' has no orbital elements", body_id),
                })?;
            let parent_mass = calculate_parent_effective_mass(
                &mp.orbital_parent(),
                &star_ref_map,
                &planet_ref_map,
                &bary_ref_map,
                &minor_planet_ref_map,
            )?;
            let mu = combined_gravitational_parameter(mp.mass(), parent_mass);
            let n = mean_motion(elements.semi_major_axis(), mu);
            let period = orbital_period(elements.semi_major_axis(), mu).ok_or_else(|| {
                DomainError::InvalidInvariant {
                    field: "orbital_period".to_string(),
                    reason: format!("invalid orbital period for minor planet '{}'", body_id),
                }
            })?;
            Ok(BodyOrbitInfo {
                elements,
                mean_motion: n,
                period,
            })
        } else {
            Err(DomainError::InvalidInvariant {
                field: "entity_id".to_string(),
                reason: format!("entity '{}' not found in system hierarchy", body_id),
            }
            .into())
        }
    }
}

pub async fn load_system_hierarchy(
    pool: &SqlitePool,
    star_system_id: &Uuid,
) -> AppResult<SystemHierarchyContext> {
    let (stars, planets, barycenters, minor_planets) =
        fetch_system_hierarchy(pool, star_system_id).await?;

    let star_map = stars.into_iter().map(|s| (s.id(), s)).collect();
    let planet_map = planets.into_iter().map(|p| (p.id(), p)).collect();
    let barycenter_map = barycenters.into_iter().map(|b| (b.id(), b)).collect();
    let minor_planet_map = minor_planets.into_iter().map(|mp| (mp.id(), mp)).collect();

    Ok(SystemHierarchyContext {
        star_map,
        planet_map,
        barycenter_map,
        minor_planet_map,
    })
}
