use crate::error::DbResult;
use crate::models::{HydrosphereComponentRow, HydrosphereRow};
use crate::repositories::fetch::{
    fetch_all_by_param, fetch_all_by_param_on_connection, fetch_optional_by_param,
    fetch_optional_by_param_on_connection,
};
use astronomicon_core::domain::{Hydrosphere, HydrosphereComponent};
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, planet_id, average_depth_m, surface_coverage_fraction, salinity_or_solute_mass_fraction \
    FROM hydrospheres WHERE planet_id = ?";
const COMPONENT_QUERY: &str = "SELECT formula, percentage \
    FROM hydrosphere_components WHERE hydrosphere_id = ?";

pub async fn get_by_planet_id(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> DbResult<Option<Hydrosphere>> {
    let base_row =
        fetch_optional_by_param::<HydrosphereRow, _>(pool, BASE_QUERY, planet_id.to_string())
            .await?;

    let row = match base_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let comp_rows =
        fetch_all_by_param::<HydrosphereComponentRow, _>(pool, COMPONENT_QUERY, &row.id).await?;

    assemble_hydrosphere(row, comp_rows)
}

pub async fn get_by_planet_id_on_connection(
    connection: &mut SqliteConnection,
    planet_id: &Uuid,
) -> DbResult<Option<Hydrosphere>> {
    let base_row = fetch_optional_by_param_on_connection::<HydrosphereRow, _>(
        connection,
        BASE_QUERY,
        planet_id.to_string(),
    )
    .await?;
    let Some(row) = base_row else { return Ok(None) };
    let comp_rows = fetch_all_by_param_on_connection::<HydrosphereComponentRow, _>(
        connection,
        COMPONENT_QUERY,
        &row.id,
    )
    .await?;
    assemble_hydrosphere(row, comp_rows)
}

fn assemble_hydrosphere(
    row: HydrosphereRow,
    comp_rows: Vec<HydrosphereComponentRow>,
) -> DbResult<Option<Hydrosphere>> {
    let mut components = Vec::with_capacity(comp_rows.len());
    for comp in comp_rows {
        components.push(HydrosphereComponent::new(comp.formula, comp.percentage)?);
    }

    let hydrosphere = row.to_domain(components)?;

    Ok(Some(hydrosphere))
}
