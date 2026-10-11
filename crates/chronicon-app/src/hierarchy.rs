use crate::error::{AppError, AppResult};
use astronomicon_core::domain::{
    Barycenter, MinorPlanet, OrbitalElements, OrbitalParent, Planet, Star,
};
use astronomicon_core::error::DomainResult;
use astronomicon_core::units::Mass;
use astronomicon_db::repositories::{
    barycenter as barycenter_repo, minor_planet as minor_planet_repo, planet as planet_repo,
    star as star_repo,
};
use chronicon_core::domain::{
    analyze_polysolar_day, PolysolarDayAnalysis, SatelliteInput,
};
use chronicon_core::math::AxialPerturber;
use sqlx::SqlitePool;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SatelliteData {
    pub id: Uuid,
    pub name: String,
    pub mass: Mass,
    pub orbital_elements: OrbitalElements,
}

impl SatelliteData {
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        mass: Mass,
        orbital_elements: OrbitalElements,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            mass,
            orbital_elements,
        }
    }

    pub fn to_input(&self) -> SatelliteInput<'_> {
        SatelliteInput::new(self.id, &self.name, self.mass, &self.orbital_elements)
    }

    pub fn to_axial_perturber(&self) -> AxialPerturber {
        AxialPerturber::new(
            self.mass,
            self.orbital_elements.semi_major_axis(),
            self.orbital_elements.eccentricity(),
        )
    }
}

pub struct LoadedPlanetHierarchy {
    pub target_planet: Planet,
    pub target_elements: OrbitalElements,
    pub stars: HashMap<Uuid, Star>,
    pub planets: HashMap<Uuid, Planet>,
    pub barycenters: HashMap<Uuid, Barycenter>,
    pub minor_planets: HashMap<Uuid, MinorPlanet>,
    pub satellites: Vec<SatelliteData>,
}

impl LoadedPlanetHierarchy {
    pub fn stars_ref_map(&self) -> HashMap<Uuid, &Star> {
        self.stars.iter().map(|(id, star)| (*id, star)).collect()
    }

    pub fn planets_ref_map(&self) -> HashMap<Uuid, &Planet> {
        self.planets.iter().map(|(id, planet)| (*id, planet)).collect()
    }

    pub fn barycenters_ref_map(&self) -> HashMap<Uuid, &Barycenter> {
        self.barycenters
            .iter()
            .map(|(id, bary)| (*id, bary))
            .collect()
    }

    pub fn minor_planets_ref_map(&self) -> HashMap<Uuid, &MinorPlanet> {
        self.minor_planets
            .iter()
            .map(|(id, minor)| (*id, minor))
            .collect()
    }

    pub fn satellite_inputs(&self) -> Vec<SatelliteInput<'_>> {
        self.satellites.iter().map(|s| s.to_input()).collect()
    }

    pub fn satellite_perturbers(&self) -> Vec<AxialPerturber> {
        self.satellites
            .iter()
            .map(|s| s.to_axial_perturber())
            .collect()
    }

    pub fn find_parent_star(&self) -> Option<&Star> {
        let mut current_parent = self.target_planet.orbital_parent();
        loop {
            match current_parent {
                OrbitalParent::Star(star_id) => return self.stars.get(&star_id),
                OrbitalParent::Barycenter(bary_id) => {
                    let bary = self.barycenters.get(&bary_id)?;
                    let primary_id = bary.member_primary().id();
                    if let Some(star) = self.stars.get(&primary_id) {
                        return Some(star);
                    }
                    current_parent = bary.orbital_parent();
                }
                OrbitalParent::Planet(planet_id) => {
                    let p = self.planets.get(&planet_id)?;
                    current_parent = p.orbital_parent();
                }
                OrbitalParent::MinorPlanet(minor_id) => {
                    let mp = self.minor_planets.get(&minor_id)?;
                    current_parent = mp.orbital_parent();
                }
                OrbitalParent::Fixed => return None,
            }
        }
    }

    pub fn analyze_polysolar_day(
        &self,
        max_denominator: Option<u32>,
    ) -> DomainResult<PolysolarDayAnalysis> {
        let stars = self.stars_ref_map();
        let planets = self.planets_ref_map();
        let barycenters = self.barycenters_ref_map();
        let minor_planets = self.minor_planets_ref_map();

        analyze_polysolar_day(
            &self.target_planet,
            &stars,
            &planets,
            &barycenters,
            &minor_planets,
            max_denominator,
        )
    }
}

pub async fn load_planet_hierarchy(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> AppResult<LoadedPlanetHierarchy> {
    let target_row = planet_repo::get_by_id(pool, planet_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: "Planet".to_string(),
            id: planet_id.to_string(),
        })?;

    let target_planet = Planet::try_from(target_row)?;

    let target_elements = target_planet
        .orbital_elements()
        .ok_or_else(|| {
            AppError::Domain(format!(
                "Planet {} has no orbital elements defined",
                planet_id
            ))
        })?;

    let all_star_rows = star_repo::list_all(pool).await?;
    let all_planet_rows = planet_repo::list_all(pool).await?;
    let all_barycenter_rows = barycenter_repo::list_all(pool).await?;
    let all_minor_planet_rows = minor_planet_repo::list_all(pool).await?;

    let mut stars = HashMap::with_capacity(all_star_rows.len());
    for row in all_star_rows {
        let star = Star::try_from(row)?;
        stars.insert(star.id(), star);
    }

    let mut planets = HashMap::with_capacity(all_planet_rows.len());
    for row in all_planet_rows {
        let planet = Planet::try_from(row)?;
        planets.insert(planet.id(), planet);
    }

    let mut barycenters = HashMap::with_capacity(all_barycenter_rows.len());
    for row in all_barycenter_rows {
        let bary = Barycenter::try_from(row)?;
        barycenters.insert(bary.id(), bary);
    }

    let mut minor_planets = HashMap::with_capacity(all_minor_planet_rows.len());
    for row in all_minor_planet_rows {
        let minor = MinorPlanet::try_from(row)?;
        minor_planets.insert(minor.id(), minor);
    }

    let mut satellites = Vec::new();
    let target_id = *planet_id;

    for planet in planets.values() {
        if planet.id() != target_id && planet.orbital_parent() == OrbitalParent::Planet(target_id) {
            if let Some(elem) = planet.orbital_elements() {
                satellites.push(SatelliteData::new(
                    planet.id(),
                    planet.name(),
                    planet.mass(),
                    elem,
                ));
            }
        }
    }

    for minor in minor_planets.values() {
        if minor.orbital_parent() == OrbitalParent::Planet(target_id) {
            if let Some(elem) = minor.orbital_elements() {
                satellites.push(SatelliteData::new(
                    minor.id(),
                    minor.name(),
                    minor.mass(),
                    elem,
                ));
            }
        }
    }

    Ok(LoadedPlanetHierarchy {
        target_planet,
        target_elements,
        stars,
        planets,
        barycenters,
        minor_planets,
        satellites,
    })
}