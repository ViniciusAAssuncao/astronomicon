use crate::error::RocketDbError;
use astronomicon_core::units::{Duration, Energy};
use rocketcon_core::domain::EnergyReservoirState;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct EnergyReservoirStateRow {
    pub vehicle_component_id: String,
    pub stored_energy_j: f64,
    pub captured_universe_epoch_s: f64,
    pub captured_at_epoch_s: f64,
}

impl TryFrom<EnergyReservoirStateRow> for EnergyReservoirState {
    type Error = RocketDbError;

    fn try_from(row: EnergyReservoirStateRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.vehicle_component_id)?;
        let state = EnergyReservoirState::new(
            id,
            Energy::new(row.stored_energy_j),
            Duration::new(row.captured_universe_epoch_s),
            Duration::new(row.captured_at_epoch_s),
        )?;
        Ok(state)
    }
}
