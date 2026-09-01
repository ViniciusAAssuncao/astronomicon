use crate::error::RocketDbError;
use astronomicon_core::units::Mass;
use rocketcon_core::domain::PropellantTankSpecification;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentPropellantTankRow {
    pub component_id: String,
    pub propellant_id: String,
    pub max_propellant_mass_kg: f64,
}

impl TryFrom<ComponentPropellantTankRow> for PropellantTankSpecification {
    type Error = RocketDbError;

    fn try_from(row: ComponentPropellantTankRow) -> Result<Self, Self::Error> {
        let component_id = Uuid::parse_str(&row.component_id)?;
        let propellant_id = Uuid::parse_str(&row.propellant_id)?;

        let spec = PropellantTankSpecification::new(
            component_id,
            propellant_id,
            Mass::new(row.max_propellant_mass_kg),
        )?;

        Ok(spec)
    }
}
