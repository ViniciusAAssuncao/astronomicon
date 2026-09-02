use crate::error::RocketDbError;
use astronomicon_core::units::{AngularMomentum, Torque};
use rocketcon_core::domain::ReactionWheelSpecification;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentReactionWheelRow {
    pub component_id: String,
    pub max_torque_n_m: f64,
    pub max_angular_momentum_storage_n_m_s: f64,
}

impl TryFrom<ComponentReactionWheelRow> for ReactionWheelSpecification {
    type Error = RocketDbError;

    fn try_from(row: ComponentReactionWheelRow) -> Result<Self, Self::Error> {
        let component_id = Uuid::parse_str(&row.component_id)?;

        let spec = ReactionWheelSpecification::new(
            component_id,
            Torque::new(row.max_torque_n_m),
            AngularMomentum::new(row.max_angular_momentum_storage_n_m_s),
        )?;

        Ok(spec)
    }
}
