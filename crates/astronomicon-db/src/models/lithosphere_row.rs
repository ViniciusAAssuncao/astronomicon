use crate::error::DbError;
use astronomicon_core::domain::{LithosphereComponent, MaterialProperties};
use astronomicon_core::units::{Density, Pressure, Temperature};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct LithosphereJoinRow {
    pub material_id: String,
    pub percentage: f64,
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

impl LithosphereJoinRow {
    pub fn to_component(&self) -> Result<LithosphereComponent, DbError> {
        let mat_id = Uuid::parse_str(&self.material_id)?;
        let material = MaterialProperties::new(
            mat_id,
            self.name.clone(),
            Density::new(self.density_kg_per_m3),
            Pressure::new(self.shear_modulus_pa),
            Pressure::new(self.base_yield_stress_pa),
            self.thermal_conductivity_w_per_m_k,
            self.specific_heat_capacity_j_per_kg_k,
            self.thermal_expansion_per_k,
            Temperature::new(self.solidus_temperature_k),
            Temperature::new(self.liquidus_temperature_k),
            self.refractive_index_real,
            self.refractive_index_imag,
        )?;
        let component = LithosphereComponent::new(material, self.percentage)?;
        Ok(component)
    }
}
