use crate::error::DbResult;
use crate::models::{AtmosphereGasComponentRow, AtmosphereRow};
use crate::repositories::fetch::{fetch_all_by_param, fetch_optional_by_param};
use astronomicon_core::domain::{Atmosphere, GasComponent};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_planet_id(pool: &SqlitePool, planet_id: &Uuid) -> DbResult<Option<Atmosphere>> {
    let base_row = fetch_optional_by_param::<AtmosphereRow, _>(
        pool,
        "SELECT id, planet_id, pressure_pa, greenhouse_effect_k, lapse_rate_k_per_m, \
         surface_humidity, cloud_coverage_fraction, cloud_condensation_nuclei_factor \
         FROM atmospheres WHERE planet_id = ?",
        planet_id.to_string(),
    )
    .await?;

    let row = match base_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let comp_rows = fetch_all_by_param::<AtmosphereGasComponentRow, _>(
        pool,
        "SELECT atmosphere_id, formula, percentage \
         FROM atmosphere_gas_components WHERE atmosphere_id = ?",
        &row.id,
    )
    .await?;

    let mut components = Vec::with_capacity(comp_rows.len());
    for comp in comp_rows {
        components.push(GasComponent::new(comp.formula, comp.percentage)?);
    }

    let atmosphere = row.to_domain(components)?;

    Ok(Some(atmosphere))
}