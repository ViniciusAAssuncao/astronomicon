use crate::error::DbResult;
use crate::models::{HydrosphereComponentRow, HydrosphereRow};
use crate::repositories::fetch::{fetch_all_by_param, fetch_optional_by_param};
use astronomicon_core::domain::{Hydrosphere, HydrosphereComponent};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_planet_id(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> DbResult<Option<Hydrosphere>> {
    let base_row = fetch_optional_by_param::<HydrosphereRow, _>(
        pool,
        "SELECT id, planet_id, average_depth_m, surface_coverage_fraction, salinity_or_solute_mass_fraction \
         FROM hydrospheres WHERE planet_id = ?",
        planet_id.to_string(),
    )
    .await?;

    let row = match base_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let comp_rows = fetch_all_by_param::<HydrosphereComponentRow, _>(
        pool,
        "SELECT formula, percentage \
         FROM hydrosphere_components WHERE hydrosphere_id = ?",
        &row.id,
    )
    .await?;

    let mut components = Vec::with_capacity(comp_rows.len());
    for comp in comp_rows {
        components.push(HydrosphereComponent::new(comp.formula, comp.percentage)?);
    }

    let hydrosphere = row.to_domain(components)?;

    Ok(Some(hydrosphere))
}
