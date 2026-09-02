use crate::error::RocketDbError;
use astronomicon_core::units::{Duration, Force, Impulse};
use rocketcon_core::domain::ReactionControlThrusterSpecification;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentReactionControlThrusterRow {
    pub component_id: String,
    pub propellant_id: String,
    pub specific_impulse_vacuum_s: f64,
    pub max_thrust_n: f64,
    pub min_impulse_bit_n_s: Option<f64>,
}

impl TryFrom<ComponentReactionControlThrusterRow> for ReactionControlThrusterSpecification {
    type Error = RocketDbError;

    fn try_from(row: ComponentReactionControlThrusterRow) -> Result<Self, Self::Error> {
        let component_id = Uuid::parse_str(&row.component_id)?;
        let propellant_id = Uuid::parse_str(&row.propellant_id)?;
        let min_impulse_bit = row.min_impulse_bit_n_s.map(Impulse::new);

        let spec = ReactionControlThrusterSpecification::new(
            component_id,
            propellant_id,
            Duration::new(row.specific_impulse_vacuum_s),
            Force::new(row.max_thrust_n),
            min_impulse_bit,
        )?;

        Ok(spec)
    }
}
