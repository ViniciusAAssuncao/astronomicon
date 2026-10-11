use crate::error::RocketDbError;
use astronomicon_core::units::{Duration, Temperature};
use rocketcon_core::domain::ThermalNodeState;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ThermalNodeStateRow {
    pub vehicle_component_id: String,
    pub current_temperature_k: f64,
    pub captured_universe_epoch_s: f64,
    pub captured_at_epoch_s: f64,
}

impl TryFrom<ThermalNodeStateRow> for ThermalNodeState {
    type Error = RocketDbError;

    fn try_from(row: ThermalNodeStateRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.vehicle_component_id)?;
        let state = ThermalNodeState::new(
            id,
            Temperature::new(row.current_temperature_k),
            Duration::new(row.captured_universe_epoch_s),
            Duration::new(row.captured_at_epoch_s),
        )?;
        Ok(state)
    }
}
