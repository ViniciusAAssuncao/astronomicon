use crate::error::DbResult;
use crate::models::MaterialPropertiesRow;
use astronomicon_core::domain::MaterialProperties;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<MaterialProperties>> {
    let row = sqlx
        ::query_as::<_, MaterialPropertiesRow>(
            "SELECT id, name, density_kg_per_m3, shear_modulus_pa, base_yield_stress_pa, \
         thermal_conductivity_w_per_m_k, specific_heat_capacity_j_per_kg_k, \
         thermal_expansion_per_k, solidus_temperature_k, liquidus_temperature_k, \
         refractive_index_real, refractive_index_imag \
         FROM material_properties WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(pool).await?;

    row.map(MaterialProperties::try_from).transpose()
}

pub async fn get_by_name(pool: &SqlitePool, name: &str) -> DbResult<Option<MaterialProperties>> {
    let row = sqlx
        ::query_as::<_, MaterialPropertiesRow>(
            "SELECT id, name, density_kg_per_m3, shear_modulus_pa, base_yield_stress_pa, \
         thermal_conductivity_w_per_m_k, specific_heat_capacity_j_per_kg_k, \
         thermal_expansion_per_k, solidus_temperature_k, liquidus_temperature_k, \
         refractive_index_real, refractive_index_imag \
         FROM material_properties WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(pool).await?;

    row.map(MaterialProperties::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<MaterialProperties>> {
    let rows = sqlx
        ::query_as::<_, MaterialPropertiesRow>(
            "SELECT id, name, density_kg_per_m3, shear_modulus_pa, base_yield_stress_pa, \
         thermal_conductivity_w_per_m_k, specific_heat_capacity_j_per_kg_k, \
         thermal_expansion_per_k, solidus_temperature_k, liquidus_temperature_k, \
         refractive_index_real, refractive_index_imag \
         FROM material_properties ORDER BY name ASC"
        )
        .fetch_all(pool).await?;

    rows.into_iter().map(MaterialProperties::try_from).collect()
}