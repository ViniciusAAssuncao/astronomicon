use crate::error::DbError;
use astronomicon_core::domain::{Atmosphere, GasComponent};
use astronomicon_core::units::{Pressure, Temperature, TemperatureGradient};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct AtmosphereRow {
    pub id: String,
    pub planet_id: String,
    pub pressure_pa: f64,
    pub greenhouse_effect_k: f64,
    pub lapse_rate_k_per_m: f64,
    pub surface_humidity: Option<f64>,
    pub cloud_coverage_fraction: Option<f64>,
    pub cloud_condensation_nuclei_factor: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct AtmosphereGasComponentRow {
    pub atmosphere_id: String,
    pub formula: String,
    pub percentage: f64,
}

impl AtmosphereRow {
    pub fn to_domain(&self, components: Vec<GasComponent>) -> Result<Atmosphere, DbError> {
        let id = Uuid::parse_str(&self.id)?;
        let planet_id = Uuid::parse_str(&self.planet_id)?;
        let atmosphere = Atmosphere::builder(
            id,
            planet_id,
            Pressure::new(self.pressure_pa),
            Temperature::new(self.greenhouse_effect_k),
            TemperatureGradient::new(self.lapse_rate_k_per_m),
        )
        .with_composition(components)
        .with_surface_humidity(self.surface_humidity)
        .with_cloud_coverage_fraction(self.cloud_coverage_fraction)
        .with_cloud_condensation_nuclei_factor(self.cloud_condensation_nuclei_factor)
        .build()?;

        Ok(atmosphere)
    }
}