use crate::error::AppResult;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    barycenter_repository, minor_planet_repository, planet_repository, star_repository,
};
use uuid::Uuid;

pub async fn fetch_system_hierarchy(
    pool: &SqlitePool,
    star_system_id: &Uuid,
) -> AppResult<(Vec<Star>, Vec<Planet>, Vec<Barycenter>, Vec<MinorPlanet>)> {
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

    let minor_planet_rows = minor_planet_repository::list_by_system(pool, star_system_id).await?;
    let minor_planets: Vec<MinorPlanet> = minor_planet_rows
        .into_iter()
        .map(MinorPlanet::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok((stars, planets, barycenters, minor_planets))
}
