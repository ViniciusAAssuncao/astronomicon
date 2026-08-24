use crate::error::AppResult;
use crate::hierarchy::fetch_system_hierarchy;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_db::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

pub struct SystemHierarchy {
    pub stars: Vec<Star>,
    pub planets: Vec<Planet>,
    pub barycenters: Vec<Barycenter>,
    pub minor_planets: Vec<MinorPlanet>,
}

impl SystemHierarchy {
    pub async fn load(pool: &SqlitePool, star_system_id: &Uuid) -> AppResult<Self> {
        let (stars, planets, barycenters, minor_planets) =
            fetch_system_hierarchy(pool, star_system_id).await?;
        Ok(Self {
            stars,
            planets,
            barycenters,
            minor_planets,
        })
    }

    pub fn maps(
        &self,
    ) -> (
        HashMap<Uuid, &Star>,
        HashMap<Uuid, &Planet>,
        HashMap<Uuid, &Barycenter>,
        HashMap<Uuid, &MinorPlanet>,
    ) {
        let star_map = self.stars.iter().map(|s| (s.id(), s)).collect();
        let planet_map = self.planets.iter().map(|p| (p.id(), p)).collect();
        let barycenter_map = self.barycenters.iter().map(|b| (b.id(), b)).collect();
        let minor_planet_map = self.minor_planets.iter().map(|mp| (mp.id(), mp)).collect();
        (star_map, planet_map, barycenter_map, minor_planet_map)
    }
}
