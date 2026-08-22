use crate::error::DbResult;
use astronomicon_core::domain::{Hydrosphere, HydrosphereComponent};
use astronomicon_core::units::Length;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(FromRow)]
struct HydrosphereRow {
    id: String,
    planet_id: String,
    average_depth_m: f64,
    surface_coverage_fraction: f64,
    salinity_or_solute_mass_fraction: f64,
}

#[derive(FromRow)]
struct HydrosphereComponentRow {
    formula: String,
    percentage: f64,
}

pub async fn get_by_planet_id(pool: &SqlitePool, planet_id: &Uuid) -> DbResult<Option<Hydrosphere>> {
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

    let id = Uuid::parse_str(&row.id)?;
    let planet_uuid = Uuid::parse_str(&row.planet_id)?;

    let hydrosphere = Hydrosphere::new(
        id,
        planet_uuid,
        Length::new(row.average_depth_m),
        row.surface_coverage_fraction,
        row.salinity_or_solute_mass_fraction,
        components,
    )?;

    Ok(Some(hydrosphere))
}
