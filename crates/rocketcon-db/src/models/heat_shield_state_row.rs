use crate::error::RocketDbError;
use astronomicon_core::units::{Duration, Length, Temperature};
use rocketcon_core::domain::HeatShieldState;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct HeatShieldStateRow {
    pub vehicle_component_id: String,
    pub remaining_thickness_m: f64,
    pub surface_temperature_k: f64,
    pub captured_universe_epoch_s: f64,
    pub captured_at_epoch_s: f64,
}

impl TryFrom<HeatShieldStateRow> for HeatShieldState {
    type Error = RocketDbError;

    fn try_from(row: HeatShieldStateRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.vehicle_component_id)?;
        let state = HeatShieldState::new(
            id,
            Length::new(row.remaining_thickness_m),
            Temperature::new(row.surface_temperature_k),
            Duration::new(row.captured_universe_epoch_s),
            Duration::new(row.captured_at_epoch_s),
        )?;
        Ok(state)
    }
}