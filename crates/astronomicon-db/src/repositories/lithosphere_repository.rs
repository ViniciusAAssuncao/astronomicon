use crate::error::DbResult;
use crate::models::LithosphereJoinRow;
use crate::repositories::fetch::fetch_all_by_param;
use astronomicon_core::domain::PlanetRheology;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_planet_id(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> DbResult<Option<PlanetRheology>> {
    let rows = fetch_all_by_param::<LithosphereJoinRow, _>(
        pool,
        "SELECT c.material_id, c.percentage, m.name, m.density_kg_per_m3, \
         m.shear_modulus_pa, m.base_yield_stress_pa, m.thermal_conductivity_w_per_m_k, \
         m.specific_heat_capacity_j_per_kg_k, m.thermal_expansion_per_k, \
         m.solidus_temperature_k, m.liquidus_temperature_k, \
         m.refractive_index_real, m.refractive_index_imag \
         FROM planet_lithosphere_components c \
         JOIN material_properties m ON c.material_id = m.id \
         WHERE c.planet_id = ?",
        planet_id.to_string(),
    )
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut components = Vec::with_capacity(rows.len());
    for row in rows {
        components.push(row.to_component()?);
    }

    let rheology = PlanetRheology::new(components)?;
    Ok(Some(rheology))
}
