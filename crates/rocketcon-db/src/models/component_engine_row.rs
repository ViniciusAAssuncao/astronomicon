use crate::error::RocketDbError;
use astronomicon_core::units::{Angle, AngularVelocity, Duration, Force, Mass};
use rocketcon_core::domain::{EngineSpecification, IgnitionType};
use rocketcon_core::error::RocketDomainError;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct ComponentEngineRow {
    pub component_id: String,
    pub fuel_propellant_id: String,
    pub oxidizer_propellant_id: Option<String>,
    pub specific_impulse_vacuum_s: f64,
    pub specific_impulse_sea_level_s: Option<f64>,
    pub max_thrust_n: f64,
    pub ignition_type: String,
    pub integral_propellant_mass_kg: Option<f64>,
    pub max_gimbal_deflection_rad: Option<f64>,
    pub gimbal_slew_rate_rad_s: Option<f64>,
    pub min_throttle_fraction: Option<f64>,
    pub oxidizer_to_fuel_mass_ratio: Option<f64>,
}

impl TryFrom<ComponentEngineRow> for EngineSpecification {
    type Error = RocketDbError;

    fn try_from(row: ComponentEngineRow) -> Result<Self, Self::Error> {
        let component_id = Uuid::parse_str(&row.component_id)?;
        let fuel_propellant_id = Uuid::parse_str(&row.fuel_propellant_id)?;
        let oxidizer_propellant_id = row
            .oxidizer_propellant_id
            .map(|s| Uuid::parse_str(&s))
            .transpose()?;

        let ignition_type = match row.ignition_type.as_str() {
            "Restartable" => IgnitionType::Restartable,
            "SingleBurn" => IgnitionType::SingleBurn,
            other => {
                return Err(RocketDbError::Domain(RocketDomainError::InvalidInvariant {
                    field: "ignition_type".to_string(),
                    reason: format!("unknown ignition type: {}", other),
                }));
            }
        };

        let spec = EngineSpecification::builder(
            component_id,
            fuel_propellant_id,
            Duration::new(row.specific_impulse_vacuum_s),
            Force::new(row.max_thrust_n),
            ignition_type,
        )
        .with_oxidizer_propellant_id(oxidizer_propellant_id)
        .with_specific_impulse_sea_level(row.specific_impulse_sea_level_s.map(Duration::new))
        .with_integral_propellant_mass(row.integral_propellant_mass_kg.map(Mass::new))
        .with_max_gimbal_deflection(row.max_gimbal_deflection_rad.map(Angle::new))
        .with_gimbal_slew_rate(row.gimbal_slew_rate_rad_s.map(AngularVelocity::new))
        .with_min_throttle_fraction(row.min_throttle_fraction)
        .with_oxidizer_to_fuel_mass_ratio(row.oxidizer_to_fuel_mass_ratio)
        .build()?;

        Ok(spec)
    }
}