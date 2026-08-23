use crate::error::AppResult;
use astronomicon_core::domain::{
    Barycenter, BarycenterMember, MinorPlanet, OrbitalElements, Planet, Star,
};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{
    calculate_effective_mass, calculate_parent_effective_mass, combined_gravitational_parameter,
};
use astronomicon_core::math::kepler::{
    mean_longitude_at_epoch, mean_motion, orbital_period,
};
use astronomicon_core::math::resonance::{
    classify_libration, laplace_resonant_argument, mean_motion_resonance_search,
    resonance_order, resonant_argument, ResonanceState,
};
use astronomicon_core::units::{Angle, AngularVelocity, Duration};
use astronomicon_db::repositories::{
    barycenter_repository, minor_planet_repository, planet_repository, star_repository,
};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResonanceReport {
    pub p: u32,
    pub q: u32,
    pub resonance_order: u32,
    pub normalized_deviation: f64,
    pub state: ResonanceState,
    pub current_critical_angle: Angle,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaplaceChainReport {
    pub state: ResonanceState,
    pub current_critical_angle: Angle,
    pub inner_mmr: Option<ResonanceReport>,
    pub outer_mmr: Option<ResonanceReport>,
}

#[derive(Clone, Copy)]
struct BodyOrbitInfo {
    elements: OrbitalElements,
    mean_motion: AngularVelocity,
    period: Duration,
}

struct SystemHierarchyContext {
    star_map: HashMap<Uuid, Star>,
    planet_map: HashMap<Uuid, Planet>,
    barycenter_map: HashMap<Uuid, Barycenter>,
    minor_planet_map: HashMap<Uuid, MinorPlanet>,
}

impl SystemHierarchyContext {
    fn get_body_orbit_info(&self, body_id: &Uuid) -> AppResult<BodyOrbitInfo> {
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
            let elements = star
                .orbital_elements()
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
            let elements = bary
                .external_orbital_elements()
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

async fn load_system_hierarchy(
    pool: &SqlitePool,
    star_system_id: &Uuid,
) -> AppResult<SystemHierarchyContext> {
    let star_rows = star_repository::list_by_system(pool, star_system_id).await?;
    let stars: Vec<Star> = star_rows
        .into_iter()
        .map(Star::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let planet_rows = planet_repository::list_by_system(pool, star_system_id).await?;
    let planets: Vec<Planet> = planet_rows
        .into_iter()
        .map(Planet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let barycenter_rows = barycenter_repository::list_by_system(pool, star_system_id).await?;
    let barycenters: Vec<Barycenter> = barycenter_rows
        .into_iter()
        .map(Barycenter::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let minor_planet_rows =
        minor_planet_repository::list_by_system(pool, star_system_id).await?;
    let minor_planets: Vec<MinorPlanet> = minor_planet_rows
        .into_iter()
        .map(MinorPlanet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

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

pub async fn resolve_orbital_resonance(
    pool: &SqlitePool,
    star_system_id: Uuid,
    body_a_id: Uuid,
    body_b_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    samples: usize,
) -> AppResult<Option<ResonanceReport>> {
    let hierarchy = load_system_hierarchy(pool, &star_system_id).await?;
    let info_a = hierarchy.get_body_orbit_info(&body_a_id)?;
    let info_b = hierarchy.get_body_orbit_info(&body_b_id)?;

    let (inner_info, outer_info) = if info_a.mean_motion.value() >= info_b.mean_motion.value() {
        (info_a, info_b)
    } else {
        (info_b, info_a)
    };

    let max_order = 32;
    let (p, q, dev) = match mean_motion_resonance_search(
        inner_info.mean_motion,
        outer_info.mean_motion,
        max_order,
    ) {
        Some(res) => res,
        None => return Ok(None),
    };

    let order = resonance_order(p, q);
    let sample_count = samples.max(2);

    let delta_n = (inner_info.mean_motion.value() - outer_info.mean_motion.value()).abs();
    let synodic_period = if delta_n > 1e-15 {
        2.0 * PI / delta_n
    } else {
        inner_info.period.value()
    };

    let time_span = synodic_period * (p as f64).max(1.0);
    let total_epoch = universe_epoch + at_epoch;

    let mut angles = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t_offset = Duration::new((i as f64 / (sample_count - 1) as f64) * time_span);
        let current_t = total_epoch + t_offset;

        let lambda1 = mean_longitude_at_epoch(
            &inner_info.elements,
            inner_info.mean_motion,
            current_t,
        );
        let lambda2 = mean_longitude_at_epoch(
            &outer_info.elements,
            outer_info.mean_motion,
            current_t,
        );
        let varpi = inner_info.elements.longitude_of_periapsis();

        let phi = resonant_argument(p, q, lambda1, lambda2, varpi);
        angles.push(phi);
    }

    let state = classify_libration(&angles);
    let current_critical_angle = angles[0];

    Ok(Some(ResonanceReport {
        p,
        q,
        resonance_order: order,
        normalized_deviation: dev,
        state,
        current_critical_angle,
    }))
}

pub async fn resolve_laplace_chain(
    pool: &SqlitePool,
    star_system_id: Uuid,
    body_1_id: Uuid,
    body_2_id: Uuid,
    body_3_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    samples: usize,
) -> AppResult<LaplaceChainReport> {
    let hierarchy = load_system_hierarchy(pool, &star_system_id).await?;
    let info1 = hierarchy.get_body_orbit_info(&body_1_id)?;
    let info2 = hierarchy.get_body_orbit_info(&body_2_id)?;
    let info3 = hierarchy.get_body_orbit_info(&body_3_id)?;

    let mut sorted = [
        (body_1_id, info1),
        (body_2_id, info2),
        (body_3_id, info3),
    ];
    sorted.sort_by(|a, b| {
        b.1.mean_motion
            .partial_cmp(&a.1.mean_motion)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let (id1, inner) = sorted[0];
    let (id2, middle) = sorted[1];
    let (id3, outer) = sorted[2];

    let inner_mmr = resolve_orbital_resonance(
        pool,
        star_system_id,
        id1,
        id2,
        universe_epoch,
        at_epoch,
        samples,
    )
    .await?;

    let outer_mmr = resolve_orbital_resonance(
        pool,
        star_system_id,
        id2,
        id3,
        universe_epoch,
        at_epoch,
        samples,
    )
    .await?;

    let sample_count = samples.max(2);
    let delta_n = (middle.mean_motion.value() - outer.mean_motion.value()).abs();
    let synodic_period = if delta_n > 1e-15 {
        2.0 * PI / delta_n
    } else {
        middle.period.value()
    };
    let time_span = synodic_period * 4.0;
    let total_epoch = universe_epoch + at_epoch;

    let mut angles = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t_offset = Duration::new((i as f64 / (sample_count - 1) as f64) * time_span);
        let current_t = total_epoch + t_offset;

        let l1 = mean_longitude_at_epoch(&inner.elements, inner.mean_motion, current_t);
        let l2 = mean_longitude_at_epoch(&middle.elements, middle.mean_motion, current_t);
        let l3 = mean_longitude_at_epoch(&outer.elements, outer.mean_motion, current_t);

        let phi = laplace_resonant_argument(l1, l2, l3);
        angles.push(phi);
    }

    let state = classify_libration(&angles);
    let current_critical_angle = angles[0];

    Ok(LaplaceChainReport {
        state,
        current_critical_angle,
        inner_mmr,
        outer_mmr,
    })
}