use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::Luminosity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolarPanelSpecification {
    component_id: Uuid,
    surface_area_m2: f64,
    conversion_efficiency: f64,
    max_power_output: Luminosity,
    is_sun_tracking: bool,
}

impl SolarPanelSpecification {
    pub fn new(
        component_id: Uuid,
        surface_area_m2: f64,
        conversion_efficiency: f64,
        max_power_output: Luminosity,
        is_sun_tracking: bool,
    ) -> RocketDomainResult<Self> {
        validate_positive_finite(surface_area_m2, "surface_area_m2")?;
        if !conversion_efficiency.is_finite()
            || conversion_efficiency <= 0.0
            || conversion_efficiency > 1.0
        {
            return Err(RocketDomainError::InvalidInvariant {
                field: "conversion_efficiency".to_string(),
                reason: "must be in range (0, 1]".to_string(),
            });
        }
        validate_positive_finite(max_power_output.value(), "max_power_output")?;

        Ok(Self {
            component_id,
            surface_area_m2,
            conversion_efficiency,
            max_power_output,
            is_sun_tracking,
        })
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn surface_area_m2(&self) -> f64 {
        self.surface_area_m2
    }

    pub fn conversion_efficiency(&self) -> f64 {
        self.conversion_efficiency
    }

    pub fn max_power_output(&self) -> Luminosity {
        self.max_power_output
    }

    pub fn is_sun_tracking(&self) -> bool {
        self.is_sun_tracking
    }
}
