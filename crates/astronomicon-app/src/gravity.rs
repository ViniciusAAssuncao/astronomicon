use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use astronomicon_core::domain::{Barycenter, BarycenterMember, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{
    calculate_effective_mass, calculate_parent_effective_mass, combined_gravitational_parameter,
    gravitational_acceleration_at, gravitational_parameter,
};
use astronomicon_core::math::kepler::orbital_period;
use astronomicon_core::math::radiometry::mean_density;
use astronomicon_core::math::stability::{
    hill_sphere_radius, kozai_constant, kozai_critical_inclination, kozai_max_eccentricity,
    kozai_oscillation_timescale, mardling_aarseth_critical_ratio, mardling_aarseth_stability_ratio,
};
use astronomicon_core::math::tidal::{
    roche_limit_fluid, roche_limit_rigid, synchronous_orbit_radius,
};
use astronomicon_core::units::{
    AccelerationVector, Angle, Duration, Length, Mass, Position,
};
use astronomicon_db::repositories::{barycenter_repository, planet_repository, star_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BarycenterStabilityDiagnostic {
    pub actual_ratio: f64,
    pub critical_ratio: f64,
    pub is_stable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RocheLimits {
    pub rigid: Length,
    pub fluid: Length,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KozaiDiagnostic {
    pub kozai_constant: f64,
    pub critical_inclination: Angle,
    pub max_eccentricity: f64,
    pub oscillation_timescale: Duration,
    pub is_active: bool,
}

struct SystemHierarchy {
    stars: Vec<Star>,
    planets: Vec<Planet>,
    barycenters: Vec<Barycenter>,
}

impl SystemHierarchy {
    fn maps(
        &self,
    ) -> (
        HashMap<Uuid, &Star>,
        HashMap<Uuid, &Planet>,
        HashMap<Uuid, &Barycenter>,
    ) {
        let star_map = self.stars.iter().map(|s| (s.id(), s)).collect();
        let planet_map = self.planets.iter().map(|p| (p.id(), p)).collect();
        let barycenter_map = self.barycenters.iter().map(|b| (b.id(), b)).collect();
        (star_map, planet_map, barycenter_map)
    }
}

async fn fetch_system_hierarchy(
    pool: &SqlitePool,
    star_system_id: &Uuid,
) -> AppResult<SystemHierarchy> {
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

    Ok(SystemHierarchy {
        stars,
        planets,
        barycenters,
    })
}

pub async fn resolve_entity_effective_mass(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    entity_id: &Uuid,
) -> AppResult<Mass> {
    let hierarchy = fetch_system_hierarchy(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map) = hierarchy.maps();

    if hierarchy.stars.iter().any(|s| s.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Star(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map,
        )
        .map_err(Into::into)
    } else if hierarchy.planets.iter().any(|p| p.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Planet(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map,
        )
        .map_err(Into::into)
    } else if hierarchy.barycenters.iter().any(|b| b.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Barycenter(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map,
        )
        .map_err(Into::into)
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

pub async fn resolve_net_gravitational_acceleration(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    point: Position,
    at_epoch: Duration,
) -> AppResult<AccelerationVector> {
    let hierarchy = fetch_system_hierarchy(pool, star_system_id).await?;
    let positions = resolve_system_positions(pool, *star_system_id, at_epoch).await?;
    let mut sources = Vec::with_capacity(hierarchy.stars.len() + hierarchy.planets.len());

    for star in &hierarchy.stars {
        if let Some(&pos) = positions.get(&star.id()) {
            sources.push((pos, star.mass()));
        }
    }

    for planet in &hierarchy.planets {
        if let Some(&pos) = positions.get(&planet.id()) {
            sources.push((pos, planet.mass()));
        }
    }

    Ok(gravitational_acceleration_at(point, &sources))
}

pub async fn resolve_hill_sphere(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    entity_id: &Uuid,
) -> AppResult<Length> {
    let hierarchy = fetch_system_hierarchy(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map) = hierarchy.maps();

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
        )?;
        Ok(hill_sphere_radius(
            elements.semi_major_axis(),
            elements.eccentricity(),
            star.mass(),
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
    let hierarchy = fetch_system_hierarchy(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map) = hierarchy.maps();

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
    let hierarchy = fetch_system_hierarchy(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map) = hierarchy.maps();

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

pub async fn resolve_roche_limits(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    primary_id: &Uuid,
    satellite_id: &Uuid,
) -> AppResult<RocheLimits> {
    let hierarchy = fetch_system_hierarchy(pool, star_system_id).await?;

    let (primary_mass, primary_radius) =
        if let Some(star) = hierarchy.stars.iter().find(|s| s.id() == *primary_id) {
            let r = star.radius().ok_or_else(|| DomainError::InvalidInvariant {
                field: "radius".to_string(),
                reason: format!("primary star '{}' has no radius", primary_id),
            })?;
            (star.mass(), r)
        } else if let Some(planet) = hierarchy.planets.iter().find(|p| p.id() == *primary_id) {
            let r = planet
                .equatorial_radius()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "equatorial_radius".to_string(),
                    reason: format!("primary planet '{}' has no equatorial radius", primary_id),
                })?;
            (planet.mass(), r)
        } else {
            return Err(DomainError::InvalidInvariant {
                field: "primary_id".to_string(),
                reason: format!(
                    "primary entity '{}' not found in system '{}'",
                    primary_id, star_system_id
                ),
            }
            .into());
        };

    let (satellite_mass, satellite_radius) = if let Some(star) =
        hierarchy.stars.iter().find(|s| s.id() == *satellite_id)
    {
        let r = star.radius().ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: format!("satellite star '{}' has no radius", satellite_id),
        })?;
        (star.mass(), r)
    } else if let Some(planet) = hierarchy.planets.iter().find(|p| p.id() == *satellite_id) {
        let r = planet
            .equatorial_radius()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "equatorial_radius".to_string(),
                reason: format!(
                    "satellite planet '{}' has no equatorial radius",
                    satellite_id
                ),
            })?;
        (planet.mass(), r)
    } else {
        return Err(DomainError::InvalidInvariant {
            field: "satellite_id".to_string(),
            reason: format!(
                "satellite entity '{}' not found in system '{}'",
                satellite_id, star_system_id
            ),
        }
        .into());
    };

    let primary_density = mean_density(primary_mass, primary_radius);
    let satellite_density = mean_density(satellite_mass, satellite_radius);

    let rigid = roche_limit_rigid(primary_radius, primary_density, satellite_density);
    let fluid = roche_limit_fluid(primary_radius, primary_density, satellite_density);

    Ok(RocheLimits { rigid, fluid })
}

pub async fn resolve_synchronous_orbit_radius(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    primary_id: &Uuid,
) -> AppResult<Length> {
    let hierarchy = fetch_system_hierarchy(pool, star_system_id).await?;

    let (mass, rotation_period) =
        if let Some(star) = hierarchy.stars.iter().find(|s| s.id() == *primary_id) {
            let rot = star
                .rotation_period()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "rotation_period".to_string(),
                    reason: format!("star '{}' has no rotation period", primary_id),
                })?;
            (star.mass(), rot)
        } else if let Some(planet) = hierarchy.planets.iter().find(|p| p.id() == *primary_id) {
            let rot = planet
                .rotation_period()
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "rotation_period".to_string(),
                    reason: format!("planet '{}' has no rotation period", primary_id),
                })?;
            (planet.mass(), rot)
        } else {
            return Err(DomainError::InvalidInvariant {
                field: "primary_id".to_string(),
                reason: format!(
                    "entity '{}' not found in system '{}'",
                    primary_id, star_system_id
                ),
            }
            .into());
        };

    let mu = gravitational_parameter(mass);
    Ok(synchronous_orbit_radius(mu, rotation_period))
}