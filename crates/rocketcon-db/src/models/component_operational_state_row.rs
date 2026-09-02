use crate::error::RocketDbError;
use astronomicon_core::units::Duration;
use rocketcon_core::domain::ComponentOperationalState;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentOperationalStateRow {
    pub vehicle_component_id: String,
    pub load_fraction: f64,
    pub captured_universe_epoch_s: f64,
    pub captured_at_epoch_s: f64,
}

impl TryFrom<ComponentOperationalStateRow> for ComponentOperationalState {
    type Error = RocketDbError;

    fn try_from(row: ComponentOperationalStateRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.vehicle_component_id)?;
        let state = ComponentOperationalState::new(
            id,
            row.load_fraction,
            Duration::new(row.captured_universe_epoch_s),
            Duration::new(row.captured_at_epoch_s),
        )?;
        Ok(state)
    }
}
