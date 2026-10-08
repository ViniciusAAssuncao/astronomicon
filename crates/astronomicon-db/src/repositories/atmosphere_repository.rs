use crate::error::DbResult;
use crate::models::{AtmosphereGasComponentRow, AtmosphereRow};
use crate::repositories::fetch::{
    fetch_all_by_param, fetch_all_by_param_on_connection, fetch_optional_by_param,
    fetch_optional_by_param_on_connection,
};
use astronomicon_core::domain::{Atmosphere, GasComponent};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, planet_id, pressure_pa, greenhouse_effect_k, lapse_rate_k_per_m, \
    surface_humidity, cloud_coverage_fraction, cloud_condensation_nuclei_factor \
    FROM atmospheres WHERE planet_id = ?";
const COMPONENT_QUERY: &str = "SELECT atmosphere_id, formula, percentage \
    FROM atmosphere_gas_components WHERE atmosphere_id = ?";

pub async fn get_by_planet_id(pool: &SqlitePool, planet_id: &Uuid) -> DbResult<Option<Atmosphere>> {
    let base_row =
        fetch_optional_by_param::<AtmosphereRow, _>(pool, BASE_QUERY, planet_id.to_string())
            .await?;

    let row = match base_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let comp_rows =
        fetch_all_by_param::<AtmosphereGasComponentRow, _>(pool, COMPONENT_QUERY, &row.id).await?;

    assemble_atmosphere(row, comp_rows)
}

pub async fn get_by_planet_id_on_connection(
    connection: &mut SqliteConnection,
    planet_id: &Uuid,
) -> DbResult<Option<Atmosphere>> {
    let base_row = fetch_optional_by_param_on_connection::<AtmosphereRow, _>(
        connection,
        BASE_QUERY,
        planet_id.to_string(),
    )
    .await?;
    let Some(row) = base_row else { return Ok(None) };
    let comp_rows = fetch_all_by_param_on_connection::<AtmosphereGasComponentRow, _>(
        connection,
        COMPONENT_QUERY,
        &row.id,
    )
    .await?;
    assemble_atmosphere(row, comp_rows)
}

fn assemble_atmosphere(
    row: AtmosphereRow,
    comp_rows: Vec<AtmosphereGasComponentRow>,
) -> DbResult<Option<Atmosphere>> {
    let mut components = Vec::with_capacity(comp_rows.len());
    for comp in comp_rows {
        components.push(GasComponent::new(comp.formula, comp.percentage)?);
    }

    let atmosphere = row.to_domain(components)?;

    Ok(Some(atmosphere))
}
