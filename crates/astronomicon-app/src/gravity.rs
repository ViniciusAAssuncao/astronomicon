use crate::error::AppResult;
use astronomicon_core::domain::{Barycenter, BarycenterMember, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::calculate_effective_mass;
use astronomicon_core::units::Mass;
use astronomicon_db::repositories::{barycenter_repository, planet_repository, star_repository};
use astronomicon_db::SqlitePool;
use uuid::Uuid;

pub async fn resolve_entity_effective_mass(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    entity_id: &Uuid,
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

    let star_map = stars.iter().map(|s| (s.id(), s)).collect();
    let planet_map = planets.iter().map(|p| (p.id(), p)).collect();
    let barycenter_map = barycenters.iter().map(|b| (b.id(), b)).collect();

    if stars.iter().any(|s| s.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Star(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map,
        )
        .map_err(Into::into)
    } else if planets.iter().any(|p| p.id() == *entity_id) {
        calculate_effective_mass(
            &BarycenterMember::Planet(*entity_id),
            &star_map,
            &planet_map,
            &barycenter_map,
        )
        .map_err(Into::into)
    } else if barycenters.iter().any(|b| b.id() == *entity_id) {
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
            reason: format!("entity '{}' not found in system '{}'", entity_id, star_system_id),
        }
        .into())
    }
}