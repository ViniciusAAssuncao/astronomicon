use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::gravity::resolve_entity_effective_mass;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::lagrange::{
    lagrange_point_position, orbital_plane_normal, LagrangePoint,
};
use astronomicon_core::units::{Duration, Position, Vector3};
use astronomicon_db::repositories::{
    barycenter_repository, minor_planet_repository, planet_repository, star_repository,
};
use astronomicon_db::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

async fn resolve_orbital_normal(pool: &SqlitePool, secondary_id: &Uuid) -> AppResult<Vector3> {
    if let Some(row) = planet_repository::get_by_id(pool, secondary_id).await? {
        let planet = Planet::try_from(row)?;
        if let Some(elements) = planet.orbital_elements() {
            return Ok(orbital_plane_normal(&elements));
        }
    } else if let Some(row) = star_repository::get_by_id(pool, secondary_id).await? {
        let star = Star::try_from(row)?;
        if let Some(elements) = star.orbital_elements() {
            return Ok(orbital_plane_normal(&elements));
        }
    } else if let Some(row) = barycenter_repository::get_by_id(pool, secondary_id).await? {
        let bary = Barycenter::try_from(row)?;
        if let Some(elements) = bary.external_orbital_elements() {
            return Ok(orbital_plane_normal(&elements));
        }
        return Ok(orbital_plane_normal(&bary.internal_orbital_elements()));
    } else if let Some(row) = minor_planet_repository::get_by_id(pool, secondary_id).await? {
        let mp = MinorPlanet::try_from(row)?;
        if let Some(elements) = mp.orbital_elements() {
            return Ok(orbital_plane_normal(&elements));
        }
    }

    Ok(Vector3::new(0.0, 0.0, 1.0))
}

pub async fn resolve_lagrange_points(
    pool: &SqlitePool,
    star_system_id: Uuid,
    primary_id: Uuid,
    secondary_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<HashMap<LagrangePoint, Position>> {
    let total_epoch = universe_epoch + at_epoch;
    let m_primary = resolve_entity_effective_mass(pool, &star_system_id, &primary_id).await?;
    let m_secondary = resolve_entity_effective_mass(pool, &star_system_id, &secondary_id).await?;

    let positions = resolve_system_positions(pool, star_system_id, total_epoch).await?;

    let pos_primary = positions
        .get(&primary_id)
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "primary_id".to_string(),
            reason: format!("position for primary '{}' could not be resolved", primary_id),
        })?;

    let pos_secondary = positions
        .get(&secondary_id)
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "secondary_id".to_string(),
            reason: format!(
                "position for secondary '{}' could not be resolved",
                secondary_id
            ),
        })?;

    let normal = resolve_orbital_normal(pool, &secondary_id).await?;

    let points = [
        LagrangePoint::L1,
        LagrangePoint::L2,
        LagrangePoint::L3,
        LagrangePoint::L4,
        LagrangePoint::L5,
    ];

    let mut result = HashMap::with_capacity(5);
    for point in points {
        let pos = lagrange_point_position(
            point,
            pos_primary,
            pos_secondary,
            m_primary,
            m_secondary,
            normal,
        )?;
        result.insert(point, pos);
    }

    Ok(result)
}