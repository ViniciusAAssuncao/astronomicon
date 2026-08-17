use crate::error::DbResult;
use astronomicon_core::domain::{Atmosphere, GasComponent};
use astronomicon_core::units::{Pressure, Temperature, TemperatureGradient};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(FromRow)]
struct AtmosphereRow {
    id: String,
    planet_id: String,
    pressure_pa: f64,
    greenhouse_effect_k: f64,
    lapse_rate_k_per_m: f64,
}

#[derive(FromRow)]
struct GasComponentRow {
    formula: String,
    percentage: f64,
}

pub async fn get_by_planet_id(pool: &SqlitePool, planet_id: &Uuid) -> DbResult<Option<Atmosphere>> {
    let base_row = sqlx::query_as::<_, AtmosphereRow>(
        "SELECT id, planet_id, pressure_pa, greenhouse_effect_k, lapse_rate_k_per_m \
         FROM atmospheres WHERE planet_id = ?",
    )
    .bind(planet_id.to_string())
    .fetch_optional(pool)
    .await?;

    let row = match base_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let comp_rows = sqlx::query_as::<_, GasComponentRow>(
        "SELECT formula, percentage \
         FROM atmosphere_gas_components WHERE atmosphere_id = ?",
    )
    .bind(&row.id)
    .fetch_all(pool)
    .await?;

    let mut components = Vec::with_capacity(comp_rows.len());
    for comp in comp_rows {
        components.push(GasComponent::new(comp.formula, comp.percentage)?);
    }

    let id = Uuid::parse_str(&row.id)?;
    let planet_uuid = Uuid::parse_str(&row.planet_id)?;

    let atmosphere = Atmosphere::new(
        id,
        planet_uuid,
        Pressure::new(row.pressure_pa),
        Temperature::new(row.greenhouse_effect_k),
        TemperatureGradient::new(row.lapse_rate_k_per_m),
        components,
    )?;

    Ok(Some(atmosphere))
}