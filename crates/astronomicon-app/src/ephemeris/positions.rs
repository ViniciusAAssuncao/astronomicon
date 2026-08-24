use crate::ephemeris::node_resolution::resolve_node;
use crate::error::AppResult;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_core::error::{DomainError, DomainResult};
use astronomicon_core::units::{Duration, Position};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    barycenter_repository, minor_planet_repository, planet_repository, star_repository,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub fn compute_system_positions(
    stars: &[Star],
    planets: &[Planet],
    barycenters: &[Barycenter],
    minor_planets: &[MinorPlanet],
    time_since_epoch: Duration,
) -> DomainResult<HashMap<Uuid, Position>> {
    let star_map: HashMap<Uuid, &Star> = stars.iter().map(|s| (s.id(), s)).collect();
    let planet_map: HashMap<Uuid, &Planet> = planets.iter().map(|p| (p.id(), p)).collect();
    let barycenter_map: HashMap<Uuid, &Barycenter> =
        barycenters.iter().map(|b| (b.id(), b)).collect();
    let minor_planet_map: HashMap<Uuid, &MinorPlanet> =
        minor_planets.iter().map(|mp| (mp.id(), mp)).collect();

    let mut member_of: HashMap<Uuid, Uuid> = HashMap::with_capacity(barycenters.len() * 2);
    for b in barycenters {
        let pri_id = b.member_primary().id();
        let sec_id = b.member_secondary().id();

        if member_of.insert(pri_id, b.id()).is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "barycenters".to_string(),
                reason: format!("entity '{}' is a member of multiple barycenters", pri_id),
            });
        }
        if member_of.insert(sec_id, b.id()).is_some() {
            return Err(DomainError::InvalidInvariant {
                field: "barycenters".to_string(),
                reason: format!("entity '{}' is a member of multiple barycenters", sec_id),
            });
        }
    }

    let mut memo: HashMap<Uuid, Position> = HashMap::with_capacity(
        stars.len() + planets.len() + barycenters.len() + minor_planets.len(),
    );
    let mut visiting: HashSet<Uuid> = HashSet::new();

    for barycenter in barycenters {
        resolve_node(
            barycenter.id(),
            &member_of,
            &star_map,
            &planet_map,
            &barycenter_map,
            &minor_planet_map,
            &mut memo,
            &mut visiting,
            time_since_epoch,
        )?;
    }

    for star in stars {
        resolve_node(
            star.id(),
            &member_of,
            &star_map,
            &planet_map,
            &barycenter_map,
            &minor_planet_map,
            &mut memo,
            &mut visiting,
            time_since_epoch,
        )?;
    }

    for planet in planets {
        resolve_node(
            planet.id(),
            &member_of,
            &star_map,
            &planet_map,
            &barycenter_map,
            &minor_planet_map,
            &mut memo,
            &mut visiting,
            time_since_epoch,
        )?;
    }

    for minor_planet in minor_planets {
        resolve_node(
            minor_planet.id(),
            &member_of,
            &star_map,
            &planet_map,
            &barycenter_map,
            &minor_planet_map,
            &mut memo,
            &mut visiting,
            time_since_epoch,
        )?;
    }

    Ok(memo)
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
