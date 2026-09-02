use crate::error::RocketDbError;
use astronomicon_core::units::Duration;
use rocketcon_core::domain::ComponentPayloadState;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentPayloadStateRow {
    pub vehicle_component_id: String,
    pub is_deployed: i64,
    pub captured_universe_epoch_s: f64,
    pub captured_at_epoch_s: f64,
}

impl TryFrom<ComponentPayloadStateRow> for ComponentPayloadState {
    type Error = RocketDbError;

    fn try_from(row: ComponentPayloadStateRow) -> Result<Self, Self::Error> {
        let id = Uuid::parse_str(&row.vehicle_component_id)?;
        let state = ComponentPayloadState::new(
            id,
            row.is_deployed != 0,
            Duration::new(row.captured_universe_epoch_s),
            Duration::new(row.captured_at_epoch_s),
        )?;
        Ok(state)
    }
}