use crate::error::DbResult;
use crate::models::MaterialPropertiesRow;
use crate::repositories::fetch::{fetch_all, fetch_optional_by_param};
use astronomicon_core::domain::MaterialProperties;
use sqlx::SqlitePool;
use uuid::Uuid;

const BASE_QUERY: &str = "SELECT id, name, density_kg_per_m3, shear_modulus_pa, \
    base_yield_stress_pa, thermal_conductivity_w_per_m_k, specific_heat_capacity_j_per_kg_k, \
    thermal_expansion_per_k, solidus_temperature_k, liquidus_temperature_k, \
    refractive_index_real, refractive_index_imag FROM material_properties";

pub async fn get_by_id(pool: &SqlitePool, id: &Uuid) -> DbResult<Option<MaterialProperties>> {
    let query = format!("{BASE_QUERY} WHERE id = ?");
    let row =
        fetch_optional_by_param::<MaterialPropertiesRow, _>(pool, &query, id.to_string()).await?;
    row.map(MaterialProperties::try_from).transpose()
}

pub async fn get_by_name(pool: &SqlitePool, name: &str) -> DbResult<Option<MaterialProperties>> {
    let query = format!("{BASE_QUERY} WHERE name = ?");
    let row = fetch_optional_by_param::<MaterialPropertiesRow, _>(pool, &query, name).await?;
    row.map(MaterialProperties::try_from).transpose()
}

pub async fn list_all(pool: &SqlitePool) -> DbResult<Vec<MaterialProperties>> {
    let query = format!("{BASE_QUERY} ORDER BY name ASC");
    let rows = fetch_all::<MaterialPropertiesRow>(pool, &query).await?;
    rows.into_iter().map(MaterialProperties::try_from).collect()
}
