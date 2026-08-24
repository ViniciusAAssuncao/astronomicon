use crate::error::DbError;
use astronomicon_core::domain::{Hydrosphere, HydrosphereComponent};
use astronomicon_core::units::Length;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct HydrosphereRow {
    pub id: String,
    pub planet_id: String,
    pub average_depth_m: f64,
    pub surface_coverage_fraction: f64,
    pub salinity_or_solute_mass_fraction: f64,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct HydrosphereComponentRow {
    pub formula: String,
    pub percentage: f64,
}

impl HydrosphereRow {
    pub fn to_domain(&self, components: Vec<HydrosphereComponent>) -> Result<Hydrosphere, DbError> {
        let id = Uuid::parse_str(&self.id)?;
        let planet_id = Uuid::parse_str(&self.planet_id)?;
        let hydrosphere = Hydrosphere::new(
            id,
            planet_id,
            Length::new(self.average_depth_m),
            self.surface_coverage_fraction,
            self.salinity_or_solute_mass_fraction,
            components,
        )?;

        Ok(hydrosphere)
    }
}
