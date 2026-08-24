use crate::error::AppResult;
use crate::geology::resolve_planetary_geology;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::mineralogy::differentiation::resolve_planetary_differentiation;
use astronomicon_core::domain::{Planet, TectonicRegime};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::mineralogy::{NormativeMineralogy, crustal_petrology};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrustalPetrologyDiagnostic {
    pub tectonic_regime: TectonicRegime,
    pub has_water: bool,
    pub normative_mineralogy: NormativeMineralogy,
}

pub async fn resolve_crustal_petrology(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<CrustalPetrologyDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let diff_diag = resolve_planetary_differentiation(pool, planet_id).await?;
    let geology_diag = resolve_planetary_geology(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let has_water = hydro_diag
        .map(|h| h.liquid_depth.value() > 0.0 || h.ice_thickness.value() > 0.0)
        .unwrap_or(false)
        || planet.mantle_hydration_fraction().unwrap_or(0.0) > 0.001;

    let mineralogy = crustal_petrology(
        &diff_diag.mantle_composition,
        geology_diag.tectonic_regime,
        has_water,
    );

    Ok(CrustalPetrologyDiagnostic {
        tectonic_regime: geology_diag.tectonic_regime,
        has_water,
        normative_mineralogy: mineralogy,
    })
}
