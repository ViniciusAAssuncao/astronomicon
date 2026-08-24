use crate::error::AppResult;
use crate::mineralogy::composition::resolve_planetary_bulk_composition;
use astronomicon_core::chemistry::{
    ElementalAbundance, element_mass_fraction, fe_si_molar_ratio, mg_number, mg_si_molar_ratio,
};
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::mineralogy::differentiate_core_mantle;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    let mantle_mg_number = mg_number(&mantle_comp);

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
        mantle_mg_number,
    })
}
