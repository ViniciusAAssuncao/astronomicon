use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use astronomicon_core::chemistry::{
    c_o_molar_ratio, fe_si_molar_ratio, mg_si_molar_ratio, refractory_mass_fraction,
    solar_abundance_to_mass_fractions, volatile_mass_fraction, ElementalAbundance,
};
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::mineralogy::{
    disk_temperature_at_orbit, planetary_bulk_composition_from_disk_temp,
};
use astronomicon_core::math::radiometry::stellar_luminosity;
use astronomicon_core::units::constants::{ASTRONOMICAL_UNIT, SOLAR_LUMINOSITY, SOLAR_RADIUS};
use astronomicon_core::units::{Length, Luminosity, Temperature};
use astronomicon_db::repositories::planet_repository;
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryBulkCompositionDiagnostic {
    pub disk_temperature: Temperature,
    pub abundances: Vec<ElementalAbundance>,
    pub refractory_fraction: f64,
    pub volatile_fraction: f64,
    pub mg_si_ratio: f64,
    pub fe_si_ratio: f64,
    pub c_o_ratio: f64,
}

pub async fn resolve_planetary_bulk_composition(
    pool: &SqlitePool,
    planet_id: Uuid,
) -> AppResult<PlanetaryBulkCompositionDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, planet.orbital_parent()).await?;

    let feh = star.metallicity().unwrap_or(0.0);
    let star_temp = star
        .effective_temperature()
        .unwrap_or_else(|| Temperature::new(5778.0));
    let star_radius = star.radius().unwrap_or_else(|| Length::new(SOLAR_RADIUS));

    let star_lum = if star_radius.value() > 0.0 && star_temp.value() > 0.0 {
        stellar_luminosity(star_radius, star_temp)
    } else {
        Luminosity::new(SOLAR_LUMINOSITY)
    };

    let semi_major_axis = planet
        .orbital_elements()
        .map(|e| e.semi_major_axis())
        .unwrap_or_else(|| Length::new(ASTRONOMICAL_UNIT));

    let disk_temp = disk_temperature_at_orbit(star_lum, semi_major_axis);
    let star_abundances = solar_abundance_to_mass_fractions(feh);
    let planet_abundances = planetary_bulk_composition_from_disk_temp(
        &star_abundances,
        disk_temp,
        planet.kind(),
    );

    let refractory = refractory_mass_fraction(&planet_abundances);
    let volatile = volatile_mass_fraction(&planet_abundances);
    let mg_si = mg_si_molar_ratio(&planet_abundances);
    let fe_si = fe_si_molar_ratio(&planet_abundances);
    let c_o = c_o_molar_ratio(&planet_abundances);

    Ok(PlanetaryBulkCompositionDiagnostic {
        disk_temperature: disk_temp,
        abundances: planet_abundances,
        refractory_fraction: refractory,
        volatile_fraction: volatile,
        mg_si_ratio: mg_si,
        fe_si_ratio: fe_si,
        c_o_ratio: c_o,
    })
}