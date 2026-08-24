use crate::error::AppResult;
use astronomicon_core::domain::{
    Barycenter, BarycenterMember, MinorPlanet, OrbitalParent, Planet, Star,
};
use astronomicon_core::error::DomainError;
use astronomicon_db::repositories::{
    barycenter_repository, minor_planet_repository, planet_repository, star_repository,
};
use astronomicon_db::SqlitePool;
use std::collections::HashSet;
use uuid::Uuid;

pub async fn collect_stars_from_barycenter(
    pool: &SqlitePool,
    barycenter_id: &Uuid,
    visited: &mut HashSet<Uuid>,
) -> AppResult<Vec<Star>> {
    if !visited.insert(*barycenter_id) {
        return Err(DomainError::InvalidInvariant {
            field: "barycenter".to_string(),
            reason: format!("circular reference detected in barycenter '{}'", barycenter_id),
        }
        .into());
    }

    let row = barycenter_repository::get_by_id(pool, barycenter_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "barycenter_id".to_string(),
            reason: format!("barycenter '{}' not found", barycenter_id),
        })?;
    let barycenter = Barycenter::try_from(row)?;

    let mut stars = Vec::new();

    for member in [barycenter.member_primary(), barycenter.member_secondary()] {
        match member {
            BarycenterMember::Star(star_id) => {
                let star_row = star_repository::get_by_id(pool, &star_id)
                    .await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "star_id".to_string(),
                        reason: format!("star '{}' in barycenter not found", star_id),
                    })?;
                stars.push(Star::try_from(star_row)?);
            }
            BarycenterMember::Planet(_) => {}
            BarycenterMember::Barycenter(sub_id) => {
                let mut sub_stars =
                    Box::pin(collect_stars_from_barycenter(pool, &sub_id, visited)).await?;
                stars.append(&mut sub_stars);
            }
        }
    }

    visited.remove(barycenter_id);
    Ok(stars)
}

pub async fn find_parent_star(
    pool: &SqlitePool,
    mut current_parent: OrbitalParent,
) -> AppResult<Star> {
    let mut visited_barycenters = HashSet::new();

    loop {
        match current_parent {
            OrbitalParent::Star(star_id) => {
                let row = star_repository::get_by_id(pool, &star_id)
                    .await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_star_id".to_string(),
                        reason: format!("parent star '{}' not found", star_id),
                    })?;
                return Ok(Star::try_from(row)?);
            }
            OrbitalParent::Planet(planet_id) => {
                let row = planet_repository::get_by_id(pool, &planet_id)
                    .await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_planet_id".to_string(),
                        reason: format!("parent planet '{}' not found", planet_id),
                    })?;
                let parent_planet = Planet::try_from(row)?;
                current_parent = parent_planet.orbital_parent();
            }
            OrbitalParent::MinorPlanet(mp_id) => {
                let row = minor_planet_repository::get_by_id(pool, &mp_id)
                    .await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_minor_planet_id".to_string(),
                        reason: format!("parent minor planet '{}' not found", mp_id),
                    })?;
                let parent_mp = MinorPlanet::try_from(row)?;
                current_parent = parent_mp.orbital_parent();
            }
            OrbitalParent::Barycenter(barycenter_id) => {
                let stars = collect_stars_from_barycenter(
                    pool,
                    &barycenter_id,
                    &mut visited_barycenters,
                )
                .await?;
                let most_massive = stars
                    .into_iter()
                    .max_by(|a, b| {
                        a.mass()
                            .partial_cmp(&b.mass())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "barycenter_stars".to_string(),
                        reason: format!(
                            "no stars found in barycenter '{}' hierarchy",
                            barycenter_id
                        ),
                    })?;
                return Ok(most_massive);
            }
            OrbitalParent::Fixed => {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_hierarchy".to_string(),
                    reason: "entity has no parent star in hierarchy".to_string(),
                }
                .into());
            }
        }
    }
}

pub async fn find_companion_star(pool: &SqlitePool, star: &Star) -> AppResult<Option<Star>> {
    match star.orbital_parent() {
        OrbitalParent::Barycenter(bary_id) => {
            let row = barycenter_repository::get_by_id(pool, &bary_id)
                .await?
                .ok_or_else(|| DomainError::InvalidInvariant {
                    field: "barycenter_id".to_string(),
                    reason: format!("barycenter '{}' not found", bary_id),
                })?;
            let barycenter = Barycenter::try_from(row)?;

            let companion_id = match (barycenter.member_primary(), barycenter.member_secondary()) {
                (BarycenterMember::Star(id1), BarycenterMember::Star(id2)) => {
                    if id1 == star.id() {
                        Some(id2)
                    } else if id2 == star.id() {
                        Some(id1)
                    } else {
                        None
                    }
                }
                (BarycenterMember::Star(id), BarycenterMember::Barycenter(_))
                    if id == star.id() =>
                {
                    None
                }
                (BarycenterMember::Barycenter(_), BarycenterMember::Star(id))
                    if id == star.id() =>
                {
                    None
                }
                _ => None,
            };

            if let Some(comp_id) = companion_id {
                let comp_row = star_repository::get_by_id(pool, &comp_id).await?;
                if let Some(row) = comp_row {
                    return Ok(Some(Star::try_from(row)?));
                }
            }

            Ok(None)
        }
        OrbitalParent::Star(parent_id) => {
            let row = star_repository::get_by_id(pool, &parent_id).await?;
            if let Some(r) = row {
                Ok(Some(Star::try_from(r)?))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}
