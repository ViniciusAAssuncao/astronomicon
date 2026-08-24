use crate::error::DbError;
use astronomicon_core::domain::MaterialProperties;
use astronomicon_core::units::{Density, Pressure, Temperature};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MaterialPropertiesRow {
    pub id: String,
    pub name: String,
    pub density_kg_per_m3: f64,
    pub shear_modulus_pa: f64,
    pub base_yield_stress_pa: f64,
    pub thermal_conductivity_w_per_m_k: f64,
    pub specific_heat_capacity_j_per_kg_k: f64,
    pub thermal_expansion_per_k: f64,
    pub solidus_temperature_k: f64,
    pub liquidus_temperature_k: f64,
    pub refractive_index_real: f64,
    pub refractive_index_imag: f64,
}

impl TryFrom<MaterialPropertiesRow> for MaterialProperties {
    type Error = DbError;

    fn try_from(row: MaterialPropertiesRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.id)?;
        let material = MaterialProperties::new(
            id,
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
        Ok(material)
    }
}