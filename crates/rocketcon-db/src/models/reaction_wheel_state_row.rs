use crate::error::RocketDbError;
use astronomicon_core::units::{AngularMomentum, Duration};
use rocketcon_core::domain::ReactionWheelState;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ReactionWheelStateRow {
    pub vehicle_component_id: String,
    pub stored_angular_momentum_n_m_s: f64,
    pub captured_universe_epoch_s: f64,
    pub captured_at_epoch_s: f64,
}

impl TryFrom<ReactionWheelStateRow> for ReactionWheelState {
    type Error = RocketDbError;

    fn try_from(row: ReactionWheelStateRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.vehicle_component_id)?;
        let state = ReactionWheelState::new(
            id,
            AngularMomentum::new(row.stored_angular_momentum_n_m_s),
            Duration::new(row.captured_universe_epoch_s),
            Duration::new(row.captured_at_epoch_s),
        )?;
        Ok(state)
    }
}
