use crate::error::RocketDbError;
use astronomicon_core::units::{Energy, Luminosity};
use rocketcon_core::domain::BatterySpecification;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentBatteryRow {
    pub component_id: String,
    pub capacity_j: f64,
    pub max_discharge_power_w: f64,
    pub max_charge_power_w: Option<f64>,
}

impl TryFrom<ComponentBatteryRow> for BatterySpecification {
    type Error = RocketDbError;

    fn try_from(row: ComponentBatteryRow) -> Result<Self, Self::Error> {
        let component_id = Uuid::parse_str(&row.component_id)?;

        let spec = BatterySpecification::new(
            component_id,
            Energy::new(row.capacity_j),
            Luminosity::new(row.max_discharge_power_w),
            row.max_charge_power_w.map(Luminosity::new),
        )?;

        Ok(spec)
    }
}
