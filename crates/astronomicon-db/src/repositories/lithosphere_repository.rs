use crate::error::DbResult;
use astronomicon_core::domain::{LithosphereComponent, MaterialProperties, PlanetRheology};
use astronomicon_core::units::{Density, Pressure, Temperature};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(FromRow)]
struct LithosphereJoinRow {
    material_id: String,
    percentage: f64,
    name: String,
    density_kg_per_m3: f64,
    shear_modulus_pa: f64,
    base_yield_stress_pa: f64,
    thermal_conductivity_w_per_m_k: f64,
    specific_heat_capacity_j_per_kg_k: f64,
    thermal_expansion_per_k: f64,
    solidus_temperature_k: f64,
    liquidus_temperature_k: f64,
    refractive_index_real: f64,
    refractive_index_imag: f64,
}

pub async fn get_by_planet_id(
    pool: &SqlitePool,
    planet_id: &Uuid,
) -> DbResult<Option<PlanetRheology>> {
    let rows = sqlx::query_as::<_, LithosphereJoinRow>(
        "SELECT c.material_id, c.percentage, m.name, m.density_kg_per_m3, \
         m.shear_modulus_pa, m.base_yield_stress_pa, m.thermal_conductivity_w_per_m_k, \
         m.specific_heat_capacity_j_per_kg_k, m.thermal_expansion_per_k, \
         m.solidus_temperature_k, m.liquidus_temperature_k, \
         m.refractive_index_real, m.refractive_index_imag \
         FROM planet_lithosphere_components c \
         JOIN material_properties m ON c.material_id = m.id \
         WHERE c.planet_id = ?",
    )
    .bind(planet_id.to_string())
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut components = Vec::with_capacity(rows.len());
    for row in rows {
        let mat_id = Uuid::parse_str(&row.material_id)?;
        let material = MaterialProperties::new(
            mat_id,
            row.name,
            Density::new(row.density_kg_per_m3),
            Pressure::new(row.shear_modulus_pa),
            Pressure::new(row.base_yield_stress_pa),
            row.thermal_conductivity_w_per_m_k,
            row.specific_heat_capacity_j_per_kg_k,
            row.thermal_expansion_per_k,
            Temperature::new(row.solidus_temperature_k),
            Temperature::new(row.liquidus_temperature_k),
            row.refractive_index_real,
            row.refractive_index_imag,
        )?;
        components.push(LithosphereComponent::new(material, row.percentage)?);
    }

    let rheology = PlanetRheology::new(components)?;
    Ok(Some(rheology))
}
