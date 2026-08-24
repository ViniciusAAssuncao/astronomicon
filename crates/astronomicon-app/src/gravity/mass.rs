use crate::error::AppResult;
use crate::gravity::hierarchy_view::SystemHierarchy;
use astronomicon_core::domain::BarycenterMember;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::calculate_effective_mass;
use astronomicon_core::units::Mass;
use astronomicon_db::SqlitePool;
use uuid::Uuid;

pub async fn resolve_entity_effective_mass(
    pool: &SqlitePool,
    star_system_id: &Uuid,
    entity_id: &Uuid,
) -> AppResult<Mass> {
    let hierarchy = SystemHierarchy::load(pool, star_system_id).await?;
    let (star_map, planet_map, barycenter_map, _) = hierarchy.maps();

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
    } else if let Some(mp) = hierarchy.minor_planets.iter().find(|m| m.id() == *entity_id) {
        Ok(mp.mass())
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
