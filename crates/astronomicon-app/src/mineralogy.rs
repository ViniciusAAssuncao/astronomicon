use crate::climate::find_parent_star;
use crate::error::AppResult;
use astronomicon_core::chemistry::{
    c_o_molar_ratio, element_mass_fraction, fe_si_molar_ratio, mg_number, mg_si_molar_ratio,
    refractory_mass_fraction, solar_abundance_to_mass_fractions, volatile_mass_fraction,
    ElementalAbundance,
};
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::mineralogy::{
    differentiate_core_mantle, disk_temperature_at_orbit, planetary_bulk_composition_from_disk_temp,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryDifferentiationDiagnostic {
    pub core_mass_fraction: f64,
    pub mantle_mass_fraction: f64,
    pub bulk_composition: Vec<ElementalAbundance>,
    pub core_composition: Vec<ElementalAbundance>,
    pub mantle_composition: Vec<ElementalAbundance>,
    pub core_fe_fraction: f64,
    pub core_ni_fraction: f64,
    pub mantle_mg_si_ratio: f64,
    pub mantle_fe_si_ratio: f64,
    pub mantle_mg_number: f64,
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

    let star = find_parent_star(pool, &planet).await?;

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

pub async fn resolve_planetary_differentiation(
    pool: &SqlitePool,
    planet_id: Uuid,
) -> AppResult<PlanetaryDifferentiationDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let bulk_diag = resolve_planetary_bulk_composition(pool, planet_id).await?;
    let core_mass_fraction = planet.core_mass_fraction().unwrap_or(0.0);
    let mantle_mass_fraction = (1.0 - core_mass_fraction).max(0.0);

    let (core_comp, mantle_comp) =
        differentiate_core_mantle(&bulk_diag.abundances, core_mass_fraction);

    let core_fe = element_mass_fraction(&core_comp, "Fe");
    let core_ni = element_mass_fraction(&core_comp, "Ni");
    let mantle_mg_si = mg_si_molar_ratio(&mantle_comp);
    let mantle_fe_si = fe_si_molar_ratio(&mantle_comp);
    let mantle_mg_num = mg_number(&mantle_comp);

    Ok(PlanetaryDifferentiationDiagnostic {
        core_mass_fraction,
        mantle_mass_fraction,
        bulk_composition: bulk_diag.abundances,
        core_composition: core_comp,
        mantle_composition: mantle_comp,
        core_fe_fraction: core_fe,
        core_ni_fraction: core_ni,
        mantle_mg_si_ratio: mantle_mg_si,
        mantle_fe_si_ratio: mantle_fe_si,
        mantle_mg_number: mantle_mg_num,
    })
}