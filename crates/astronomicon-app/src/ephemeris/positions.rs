use crate::ephemeris::prepared_positions::PreparedPositionLookup;
use crate::error::AppResult;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_core::error::{DomainError, DomainResult};
use astronomicon_core::units::{Duration, Position};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    barycenter_repository, minor_planet_repository, planet_repository, star_repository,
};
use std::collections::HashMap;
use uuid::Uuid;

pub fn compute_system_positions(
    stars: &[Star],
    planets: &[Planet],
    barycenters: &[Barycenter],
    minor_planets: &[MinorPlanet],
    time_since_epoch: Duration,
) -> DomainResult<HashMap<Uuid, Position>> {
    compute_system_positions_at_offset(
        stars,
        planets,
        barycenters,
        minor_planets,
        time_since_epoch,
        0.0,
    )
}

pub fn compute_system_positions_at_offset(
    stars: &[Star],
    planets: &[Planet],
    barycenters: &[Barycenter],
    minor_planets: &[MinorPlanet],
    base: Duration,
    offset_seconds: f64,
) -> DomainResult<HashMap<Uuid, Position>> {
    compute_positions(
        stars,
        planets,
        barycenters,
        minor_planets,
        base,
        offset_seconds,
        None,
    )
}

pub fn compute_selected_system_positions_at_offset(
    stars: &[Star],
    planets: &[Planet],
    barycenters: &[Barycenter],
    minor_planets: &[MinorPlanet],
    base: Duration,
    offset_seconds: f64,
    requested: &[Uuid],
) -> DomainResult<HashMap<Uuid, Position>> {
    if requested.is_empty()
        || requested
            .iter()
            .enumerate()
            .any(|(index, id)| requested[..index].contains(id))
    {
        return Err(DomainError::InvalidInvariant {
            field: "ephemeris_body_selection".into(),
            reason: "selection must contain unique body identifiers and cannot be empty".into(),
        });
    }
    compute_positions(
        stars,
        planets,
        barycenters,
        minor_planets,
        base,
        offset_seconds,
        Some(requested),
    )
}

fn compute_positions(
    stars: &[Star],
    planets: &[Planet],
    barycenters: &[Barycenter],
    minor_planets: &[MinorPlanet],
    base: Duration,
    offset_seconds: f64,
    selected: Option<&[Uuid]>,
) -> DomainResult<HashMap<Uuid, Position>> {
    if !base.value().is_finite() || !offset_seconds.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "ephemeris_time".into(),
            reason: "non-finite epoch or offset".into(),
        });
    }
    PreparedPositionLookup::new(stars, planets, barycenters, minor_planets)?.compute(
        base,
        offset_seconds,
        selected,
    )
}

pub async fn resolve_system_positions(
    pool: &SqlitePool,
    star_system_id: Uuid,
    time_since_epoch: Duration,
) -> AppResult<HashMap<Uuid, Position>> {
    let star_rows = star_repository::list_by_system(pool, &star_system_id).await?;
    let stars = star_rows
        .into_iter()
        .map(Star::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let planet_rows = planet_repository::list_by_system(pool, &star_system_id).await?;
    let planets = planet_rows
        .into_iter()
        .map(Planet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let barycenter_rows = barycenter_repository::list_by_system(pool, &star_system_id).await?;
    let barycenters = barycenter_rows
        .into_iter()
        .map(Barycenter::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let minor_planet_rows = minor_planet_repository::list_by_system(pool, &star_system_id).await?;
    let minor_planets = minor_planet_rows
        .into_iter()
        .map(MinorPlanet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    let positions = compute_system_positions(
        &stars,
        &planets,
        &barycenters,
        &minor_planets,
        time_since_epoch,
    )?;
    Ok(positions)
}
