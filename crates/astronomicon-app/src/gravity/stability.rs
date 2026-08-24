use crate::error::AppResult;
use crate::gravity::hierarchy_view::SystemHierarchy;
use astronomicon_core::domain::BarycenterMember;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{
    calculate_effective_mass, calculate_parent_effective_mass, combined_gravitational_parameter,
    gravitational_parameter,
};
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::math::stability::{
    hill_sphere_radius, kozai_constant, kozai_critical_inclination, kozai_max_eccentricity,
    kozai_oscillation_timescale, mardling_aarseth_critical_ratio,
    mardling_aarseth_stability_ratio,
};
use astronomicon_core::units::{Angle, Duration, Length};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BarycenterStabilityDiagnostic {
    pub actual_ratio: f64,
    pub critical_ratio: f64,
    pub is_stable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KozaiDiagnostic {
    pub kozai_constant: f64,
    pub critical_inclination: Angle,
    pub max_eccentricity: f64,
    pub oscillation_timescale: Duration,
    pub is_active: bool,
}

pub async fn resolve_hill_sphere(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    entity_id: &Uuid,
) -> AppResult<Length> {
    let hierarchy = SystemHierarchy::load(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map, minor_planet_map) = hierarchy.maps();

    if let Some(planet) = hierarchy.planets.iter().find(|p| p.id() == *entity_id) {
        let elements =
            planet
                .orbital_elements()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "orbital_elements".to_string(),
                    reason: format!("planet '{}' has no orbital elements", entity_id),
                })?;
        let parent_mass = calculate_parent_effective_mass(
            &planet.orbital_parent(),
            &star_map,
            &planet_map,
            &barycenter_map,
            &minor_planet_map,
        )?;
        Ok(hill_sphere_radius(
            elements.semi_major_axis(),
            elements.eccentricity(),
            planet.mass(),
            parent_mass,
        ))
    } else if let Some(star) = hierarchy.stars.iter().find(|s| s.id() == *entity_id) {
        let elements = star
            .orbital_elements()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: format!("star '{}' has no orbital elements", entity_id),
            })?;
        let parent_mass = calculate_parent_effective_mass(
            &star.orbital_parent(),
            &star_map,
            &planet_map,
            &barycenter_map,
            &minor_planet_map,
        )?;
        Ok(hill_sphere_radius(
            elements.semi_major_axis(),
            elements.eccentricity(),
            star.mass(),
            parent_mass,
        ))
    } else if let Some(mp) = hierarchy.minor_planets.iter().find(|m| m.id() == *entity_id) {
        let elements = mp
            .orbital_elements()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: format!("minor planet '{}' has no orbital elements", entity_id),
            })?;
        let parent_mass = calculate_parent_effective_mass(
            &mp.orbital_parent(),
            &star_map,
            &planet_map,
            &barycenter_map,
            &minor_planet_map,
        )?;
        Ok(hill_sphere_radius(
            elements.semi_major_axis(),
            elements.eccentricity(),
            mp.mass(),
            parent_mass,
        ))
    } else {
        Err(DomainError::InvalidInvariant {
            field: "entity_id".to_string(),
            reason: format!(
                "entity '{}' not found in system '{}'",
                entity_id, star_system_id
            ),
        }
        .into())
    }
}

pub async fn resolve_barycenter_stability(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    barycenter_id: &Uuid,
) -> AppResult<BarycenterStabilityDiagnostic> {
    let hierarchy = SystemHierarchy::load(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map, minor_planet_map) = hierarchy.maps();

    let barycenter = hierarchy
        .barycenters
        .iter()
        .find(|b| b.id() == *barycenter_id)
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "barycenter_id".to_string(),
            reason: format!(
                "barycenter '{}' not found in system '{}'",
                barycenter_id, star_system_id
            ),
        })?;

    let ext_elements = barycenter
        .external_orbital_elements()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "external_orbital_elements".to_string(),
            reason: format!(
                "barycenter '{}' has no external orbital elements",
                barycenter_id
            ),
        })?;

    let inner_mass = calculate_effective_mass(
        &BarycenterMember::Barycenter(*barycenter_id),
        &star_map,
        &planet_map,
        &barycenter_map,
    )?;

    let outer_mass = calculate_parent_effective_mass(
        &barycenter.orbital_parent(),
        &star_map,
        &planet_map,
        &barycenter_map,
        &minor_planet_map,
    )?;

    let inner_a = barycenter.internal_orbital_elements().semi_major_axis();
    let outer_a = ext_elements.semi_major_axis();
    let outer_e = ext_elements.eccentricity();
    let outer_periapsis = Length::new(outer_a.value() * (1.0 - outer_e));

    let mutual_inc = Angle::new(
        (ext_elements.inclination().value()
            - barycenter.internal_orbital_elements().inclination().value())
        .abs(),
    );

    let actual_ratio = mardling_aarseth_stability_ratio(inner_a, outer_periapsis);
    let critical_ratio =
        mardling_aarseth_critical_ratio(inner_mass, outer_mass, outer_e, mutual_inc);

    Ok(BarycenterStabilityDiagnostic {
        actual_ratio,
        critical_ratio,
        is_stable: actual_ratio >= critical_ratio,
    })
}

pub async fn resolve_kozai_lidov_diagnostic(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    barycenter_id: &Uuid,
) -> AppResult<KozaiDiagnostic> {
    let hierarchy = SystemHierarchy::load(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map, minor_planet_map) = hierarchy.maps();

    let barycenter = hierarchy
        .barycenters
        .iter()
        .find(|b| b.id() == *barycenter_id)
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "barycenter_id".to_string(),
            reason: format!(
                "barycenter '{}' not found in system '{}'",
                barycenter_id, star_system_id
            ),
        })?;

    let ext_elements = barycenter
        .external_orbital_elements()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "external_orbital_elements".to_string(),
            reason: format!(
                "barycenter '{}' has no external orbital elements",
                barycenter_id
            ),
        })?;

    let inner_elements = barycenter.internal_orbital_elements();

    let inner_mass = calculate_effective_mass(
        &BarycenterMember::Barycenter(*barycenter_id),
        &star_map,
        &planet_map,
        &barycenter_map,
    )?;

    let outer_mass = calculate_parent_effective_mass(
        &barycenter.orbital_parent(),
        &star_map,
        &planet_map,
        &barycenter_map,
        &minor_planet_map,
    )?;

    let mu_inner = gravitational_parameter(inner_mass);
    let inner_period = orbital_period(inner_elements.semi_major_axis(), mu_inner).ok_or_else(
        || DomainError::InvalidInvariant {
            field: "internal_orbital_elements".to_string(),
            reason: format!(
                "invalid internal orbital period for barycenter '{}'",
                barycenter_id
            ),
        },
    )?;

    let mu_outer = combined_gravitational_parameter(inner_mass, outer_mass);
    let outer_period = orbital_period(ext_elements.semi_major_axis(), mu_outer).ok_or_else(
        || DomainError::InvalidInvariant {
            field: "external_orbital_elements".to_string(),
            reason: format!(
                "invalid external orbital period for barycenter '{}'",
                barycenter_id
            ),
        },
    )?;

    let mutual_inc = Angle::new(
        (ext_elements.inclination().value() - inner_elements.inclination().value()).abs(),
    );

    let kozai_const = kozai_constant(inner_elements.eccentricity(), mutual_inc);
    let crit_inc = kozai_critical_inclination();
    let max_e = kozai_max_eccentricity(mutual_inc);
    let timescale = kozai_oscillation_timescale(
        inner_period,
        outer_period,
        inner_mass,
        outer_mass,
        ext_elements.eccentricity(),
    );

    let inc_val = mutual_inc.value().rem_euclid(PI);
    let is_active = inc_val >= crit_inc.value() && inc_val <= PI - crit_inc.value();

    Ok(KozaiDiagnostic {
        kozai_constant: kozai_const,
        critical_inclination: crit_inc,
        max_eccentricity: max_e,
        oscillation_timescale: timescale,
        is_active,
    })
}
