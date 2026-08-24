use crate::error::DbResult;
use crate::models::{HydrosphereComponentRow, HydrosphereRow};
use astronomicon_core::domain::{Hydrosphere, HydrosphereComponent};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_planet_id(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> DbResult<Option<Hydrosphere>> {
    let base_row = sqlx::query_as::<_, HydrosphereRow>(
        "SELECT id, planet_id, average_depth_m, surface_coverage_fraction, salinity_or_solute_mass_fraction \
         FROM hydrospheres WHERE planet_id = ?",
    )
    .bind(planet_id.to_string())
    .fetch_optional(pool)
    .await?;

    let row = match base_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let comp_rows = sqlx::query_as::<_, HydrosphereComponentRow>(
        "SELECT formula, percentage \
         FROM hydrosphere_components WHERE hydrosphere_id = ?",
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await?;

    let mut components = Vec::with_capacity(comp_rows.len());
    for comp in comp_rows {
        components.push(HydrosphereComponent::new(comp.formula, comp.percentage)?);
    }

    let hydrosphere = row.to_domain(components)?;

    Ok(Some(hydrosphere))
}
