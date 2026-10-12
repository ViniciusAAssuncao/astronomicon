use crate::constants::constants::FUNCTIONAL_SUN_APPARENT_MAGNITUDE_THRESHOLD;
use astronomicon_core::domain::{Barycenter, MinorPlanet, Planet, Star};
use astronomicon_core::error::DomainResult;
use astronomicon_core::math::gravity::calculate_absolute_position;
use astronomicon_core::math::radiometry::{
    apparent_bolometric_magnitude, orbital_irradiance, stellar_luminosity,
};
use astronomicon_core::units::{Duration, Irradiance, Length};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StarRelevance {
    pub star_id: Uuid,
    pub distance: Length,
    pub irradiance: Irradiance,
    pub apparent_magnitude: f64,
    pub is_relevant: bool,
}

pub fn evaluate_star_relevance(
    planet: &Planet,
    star: &Star,
    stars: &HashMap<Uuid, &Star>,
    planets: &HashMap<Uuid, &Planet>,
    barycenters: &HashMap<Uuid, &Barycenter>,
    minor_planets: &HashMap<Uuid, &MinorPlanet>,
) -> DomainResult<StarRelevance> {
    let t0 = Duration::new(0.0);
    let planet_pos = calculate_absolute_position(
        planet.id(),
        t0,
        stars,
        planets,
        barycenters,
        minor_planets,
    )?;
    let star_pos = calculate_absolute_position(
        star.id(),
        t0,
        stars,
        planets,
        barycenters,
        minor_planets,
    )?;

    let diff = star_pos.raw() - planet_pos.raw();
    let dist_val = diff.magnitude();
    let distance = Length::new(dist_val);

    let (irradiance, apparent_magnitude) = match (star.radius(), star.effective_temperature()) {
        (Some(radius), Some(temp)) if radius.value() > 0.0 && temp.value() > 0.0 => {
            let lum = stellar_luminosity(radius, temp);
            let irr = orbital_irradiance(lum, distance);
            let mag = apparent_bolometric_magnitude(irr);
            (irr, mag)
        }
        _ => (Irradiance::new(0.0), f64::INFINITY),
    };

    let is_relevant = apparent_magnitude <= FUNCTIONAL_SUN_APPARENT_MAGNITUDE_THRESHOLD;

    Ok(StarRelevance {
        star_id: star.id(),
        distance,
        irradiance,
        apparent_magnitude,
        is_relevant,
    })
}