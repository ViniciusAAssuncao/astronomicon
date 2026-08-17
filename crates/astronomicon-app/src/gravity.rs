use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use astronomicon_core::domain::{ Barycenter, BarycenterMember, Planet, Star };
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{
    calculate_effective_mass,
    calculate_parent_effective_mass,
    gravitational_acceleration_at,
};
use astronomicon_core::math::stability::{
    hill_sphere_radius,
    mardling_aarseth_critical_ratio,
    mardling_aarseth_stability_ratio,
};
use astronomicon_core::units::{ AccelerationVector, Angle, Duration, Length, Mass, Position };
use astronomicon_db::repositories::{ barycenter_repository, planet_repository, star_repository };
use astronomicon_db::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarycenterStabilityDiagnostic {
    pub actual_ratio: f64,
    pub critical_ratio: f64,
    pub is_stable: bool,
}

pub async fn resolve_entity_effective_mass(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    entity_id: &Uuid
) -> AppResult<Mass> {
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

    let star_map = stars
        .iter()
        .map(|s| (s.id(), s))
        .collect();
    let planet_map = planets
        .iter()
        .map(|p| (p.id(), p))
        .collect();
    let barycenter_map = barycenters
        .iter()
        .map(|b| (b.id(), b))
        .collect();

    if stars.iter().any(|s| s.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Star(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map
        ).map_err(Into::into)
    } else if planets.iter().any(|p| p.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Planet(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map
        ).map_err(Into::into)
    } else if barycenters.iter().any(|b| b.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Barycenter(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map
        ).map_err(Into::into)
    } else {
        Err(
            (DomainError::InvalidInvariant {
                field: "entity_id".to_string(),
                reason: format!("entity '{}' not found in system '{}'", entity_id, star_system_id),
            }).into()
        )
    }
}

pub async fn resolve_net_gravitational_acceleration(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    point: Position,
    at_epoch: Duration
) -> AppResult<AccelerationVector> {
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

    let positions = resolve_system_positions(pool, *star_system_id, at_epoch).await?;
    let mut sources = Vec::with_capacity(stars.len() + planets.len());

    for star in &stars {
        if let Some(&pos) = positions.get(&star.id()) {
            sources.push((pos, star.mass()));
        }
    }

    for planet in &planets {
        if let Some(&pos) = positions.get(&planet.id()) {
            sources.push((pos, planet.mass()));
        }
    }

    Ok(gravitational_acceleration_at(point, &sources))
}

pub async fn resolve_hill_sphere(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    entity_id: &Uuid
) -> AppResult<Length> {
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

    let star_map = stars
        .iter()
        .map(|s| (s.id(), s))
        .collect();
    let planet_map = planets
        .iter()
        .map(|p| (p.id(), p))
        .collect();
    let barycenter_map = barycenters
        .iter()
        .map(|b| (b.id(), b))
        .collect();

    if let Some(planet) = planets.iter().find(|p| p.id() == *entity_id) {
        let elements = planet.orbital_elements().ok_or_else(|| DomainError::InvalidInvariant {
            field: "orbital_elements".to_string(),
            reason: format!("planet '{}' has no orbital elements", entity_id),
        })?;
        let parent_mass = calculate_parent_effective_mass(
            &planet.orbital_parent(),
            &star_map,
            &planet_map,
            &barycenter_map
        )?;
        Ok(
            hill_sphere_radius(
                elements.semi_major_axis(),
                elements.eccentricity(),
                planet.mass(),
                parent_mass
            )
        )
    } else if let Some(star) = stars.iter().find(|s| s.id() == *entity_id) {
        let elements = star.orbital_elements().ok_or_else(|| DomainError::InvalidInvariant {
            field: "orbital_elements".to_string(),
            reason: format!("star '{}' has no orbital elements", entity_id),
        })?;
        let parent_mass = calculate_parent_effective_mass(
            &star.orbital_parent(),
            &star_map,
            &planet_map,
            &barycenter_map
        )?;
        Ok(
            hill_sphere_radius(
                elements.semi_major_axis(),
                elements.eccentricity(),
                star.mass(),
                parent_mass
            )
        )
    } else {
        Err(
            (DomainError::InvalidInvariant {
                field: "entity_id".to_string(),
                reason: format!("entity '{}' not found in system '{}'", entity_id, star_system_id),
            }).into()
        )
    }
}

pub async fn resolve_barycenter_stability(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    barycenter_id: &Uuid
) -> AppResult<BarycenterStabilityDiagnostic> {
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

    let star_map = stars
        .iter()
        .map(|s| (s.id(), s))
        .collect();
    let planet_map = planets
        .iter()
        .map(|p| (p.id(), p))
        .collect();
    let barycenter_map = barycenters
        .iter()
        .map(|b| (b.id(), b))
        .collect();

    let barycenter = barycenters
        .iter()
        .find(|b| b.id() == *barycenter_id)
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "barycenter_id".to_string(),
            reason: format!(
                "barycenter '{}' not found in system '{}'",
                barycenter_id,
                star_system_id
            ),
        })?;

    let ext_elements = barycenter
        .external_orbital_elements()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "external_orbital_elements".to_string(),
            reason: format!("barycenter '{}' has no external orbital elements", barycenter_id),
        })?;

    let inner_mass = calculate_effective_mass(
        &BarycenterMember::Barycenter(*barycenter_id),
        &star_map,
        &planet_map,
        &barycenter_map
    )?;

    let outer_mass = calculate_parent_effective_mass(
        &barycenter.orbital_parent(),
        &star_map,
        &planet_map,
        &barycenter_map
    )?;

    let inner_a = barycenter.internal_orbital_elements().semi_major_axis();
    let outer_a = ext_elements.semi_major_axis();
    let outer_e = ext_elements.eccentricity();
    let outer_periapsis = Length::new(outer_a.value() * (1.0 - outer_e));

    let mutual_inc = Angle::new(
        (
            ext_elements.inclination().value() -
            barycenter.internal_orbital_elements().inclination().value()
        ).abs()
    );

    let actual_ratio = mardling_aarseth_stability_ratio(inner_a, outer_periapsis);
    let critical_ratio = mardling_aarseth_critical_ratio(
        inner_mass,
        outer_mass,
        outer_e,
        mutual_inc
    );

    Ok(BarycenterStabilityDiagnostic {
        actual_ratio,
        critical_ratio,
        is_stable: actual_ratio >= critical_ratio,
    })
}
