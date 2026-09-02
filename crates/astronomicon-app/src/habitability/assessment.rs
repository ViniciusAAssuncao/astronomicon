use crate::error::AppResult;
use crate::habitability::earth_similarity::resolve_earth_similarity_index;
use crate::habitability::nutrient_availability::resolve_nutrient_limitation;
use crate::habitability::subsurface_productivity::resolve_chemosynthetic_productivity;
use crate::habitability::surface_productivity::{
    resolve_global_surface_productivity, SurfaceProductivityDiagnostic,
};
use crate::habitability::viability::{
    resolve_global_biochemical_viability, GlobalBiochemicalViabilityDiagnostic,
};
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use astronomicon_core::math::habitability::{
    evaluate_planetary_habitability, ChemosyntheticPathways, EarthSimilarityIndex,
    FirstOrderNutrientLimitation, PlanetaryHabitabilityClassification,
};
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitabilityDiagnostic {
    pub earth_similarity: Option<EarthSimilarityIndex>,
    pub classification: PlanetaryHabitabilityClassification,
    pub surface_productivity: SurfaceProductivityDiagnostic,
    pub chemosynthetic_pathways: ChemosyntheticPathways,
    pub biochemical_viability: GlobalBiochemicalViabilityDiagnostic,
    pub nutrient_limitation: FirstOrderNutrientLimitation,
}

pub async fn resolve_habitability_assessment(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<HabitabilityDiagnostic> {
    let earth_similarity =
        resolve_earth_similarity_index(pool, planet_id, universe_epoch, at_epoch)
            .await
            .ok();

    let surface_productivity =
        resolve_global_surface_productivity(pool, planet_id, universe_epoch, at_epoch, 18)
            .await?;

    let chemosynthetic_pathways =
        resolve_chemosynthetic_productivity(pool, planet_id, universe_epoch, at_epoch).await?;

    let biochemical_viability =
        resolve_global_biochemical_viability(pool, planet_id, universe_epoch, at_epoch, 18)
            .await?;

    let nutrient_limitation =
        resolve_nutrient_limitation(pool, planet_id, universe_epoch, at_epoch).await?;

    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let is_subsurface_ocean = hydro_diag
        .as_ref()
        .map(|h| h.is_subsurface_ocean)
        .unwrap_or(false);

    let subsurface_chemical_viability = if is_subsurface_ocean { 1.0 } else { 0.0 };

    let classification = evaluate_planetary_habitability(
        biochemical_viability
            .average_radiation_tolerance
            .complex_life_survival_fraction,
        biochemical_viability
            .average_chemical_tolerance
            .overall_viability,
        biochemical_viability.liquid_solvent_surface_fraction,
        nutrient_limitation.availability_factor,
        surface_productivity
            .empirical_primary_habitability
            .sph_index,
        is_subsurface_ocean,
        subsurface_chemical_viability,
        chemosynthetic_pathways.total_chemical_power,
    );

    Ok(HabitabilityDiagnostic {
        earth_similarity,
        classification,
        surface_productivity,
        chemosynthetic_pathways,
        biochemical_viability,
        nutrient_limitation,
    })
}
