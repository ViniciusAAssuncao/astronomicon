use crate::error::RocketDbError;
use astronomicon_core::units::Luminosity;
use rocketcon_core::domain::SolarPanelSpecification;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentSolarPanelRow {
    pub component_id: String,
    pub surface_area_m2: f64,
    pub conversion_efficiency: f64,
    pub max_power_output_w: f64,
    pub is_sun_tracking: i64,
}

impl TryFrom<ComponentSolarPanelRow> for SolarPanelSpecification {
    type Error = RocketDbError;

    fn try_from(row: ComponentSolarPanelRow) -> Result<Self, Self::Error> {
        let component_id = Uuid::parse_str(&row.component_id)?;

        let spec = SolarPanelSpecification::new(
            component_id,
            row.surface_area_m2,
            row.conversion_efficiency,
            Luminosity::new(row.max_power_output_w),
            row.is_sun_tracking != 0,
        )?;

        Ok(spec)
    }
}
