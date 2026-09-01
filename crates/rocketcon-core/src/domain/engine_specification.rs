use crate::domain::ignition_type::IgnitionType;
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{Duration, Force, Mass, MassRate, Speed};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct EngineSpecificationBuilder {
    component_id: Uuid,
    fuel_propellant_id: Uuid,
    oxidizer_propellant_id: Option<Uuid>,
    specific_impulse_vacuum: Duration,
    specific_impulse_sea_level: Option<Duration>,
    max_thrust: Force,
    ignition_type: IgnitionType,
    integral_propellant_mass: Option<Mass>,
}

impl EngineSpecificationBuilder {
    pub fn new(
        component_id: Uuid,
        fuel_propellant_id: Uuid,
        specific_impulse_vacuum: Duration,
        max_thrust: Force,
        ignition_type: IgnitionType,
    ) -> Self {
        Self {
            component_id,
            fuel_propellant_id,
            oxidizer_propellant_id: None,
            specific_impulse_vacuum,
            specific_impulse_sea_level: None,
            max_thrust,
            ignition_type,
            integral_propellant_mass: None,
        }
    }

    pub fn with_oxidizer_propellant_id(
        mut self,
        oxidizer_propellant_id: impl Into<Option<Uuid>>,
    ) -> Self {
        self.oxidizer_propellant_id = oxidizer_propellant_id.into();
        self
    }

    pub fn with_specific_impulse_sea_level(
        mut self,
        specific_impulse_sea_level: impl Into<Option<Duration>>,
    ) -> Self {
        self.specific_impulse_sea_level = specific_impulse_sea_level.into();
        self
    }

    pub fn with_integral_propellant_mass(
        mut self,
        integral_propellant_mass: impl Into<Option<Mass>>,
    ) -> Self {
        self.integral_propellant_mass = integral_propellant_mass.into();
        self
    }

    pub fn build(self) -> RocketDomainResult<EngineSpecification> {
        validate_positive_finite(
            self.specific_impulse_vacuum.value(),
            "specific_impulse_vacuum",
        )?;
        validate_positive_finite(self.max_thrust.value(), "max_thrust")?;

        if let Some(isp_sl) = self.specific_impulse_sea_level {
            validate_positive_finite(isp_sl.value(), "specific_impulse_sea_level")?;
            if isp_sl.value() > self.specific_impulse_vacuum.value() {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "specific_impulse_sea_level".to_string(),
                    reason: "cannot be greater than specific_impulse_vacuum".to_string(),
                });
            }
        }

        if let Some(m) = self.integral_propellant_mass {
            validate_positive_finite(m.value(), "integral_propellant_mass")?;
            if self.ignition_type != IgnitionType::SingleBurn {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "integral_propellant_mass".to_string(),
                    reason: "integral propellant mass is only allowed for SingleBurn engines"
                        .to_string(),
                });
            }
        }

        Ok(EngineSpecification {
            component_id: self.component_id,
            fuel_propellant_id: self.fuel_propellant_id,
            oxidizer_propellant_id: self.oxidizer_propellant_id,
            specific_impulse_vacuum: self.specific_impulse_vacuum,
            specific_impulse_sea_level: self.specific_impulse_sea_level,
            max_thrust: self.max_thrust,
            ignition_type: self.ignition_type,
            integral_propellant_mass: self.integral_propellant_mass,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSpecification {
    component_id: Uuid,
    fuel_propellant_id: Uuid,
    oxidizer_propellant_id: Option<Uuid>,
    specific_impulse_vacuum: Duration,
    specific_impulse_sea_level: Option<Duration>,
    max_thrust: Force,
    ignition_type: IgnitionType,
    integral_propellant_mass: Option<Mass>,
}

impl EngineSpecification {
    pub fn builder(
        component_id: Uuid,
        fuel_propellant_id: Uuid,
        specific_impulse_vacuum: Duration,
        max_thrust: Force,
        ignition_type: IgnitionType,
    ) -> EngineSpecificationBuilder {
        EngineSpecificationBuilder::new(
            component_id,
            fuel_propellant_id,
            specific_impulse_vacuum,
            max_thrust,
            ignition_type,
        )
    }

    pub fn new(
        component_id: Uuid,
        fuel_propellant_id: Uuid,
        oxidizer_propellant_id: Option<Uuid>,
        specific_impulse_vacuum: Duration,
        specific_impulse_sea_level: Option<Duration>,
        max_thrust: Force,
        ignition_type: IgnitionType,
        integral_propellant_mass: Option<Mass>,
    ) -> RocketDomainResult<Self> {
        Self::builder(
            component_id,
            fuel_propellant_id,
            specific_impulse_vacuum,
            max_thrust,
            ignition_type,
        )
        .with_oxidizer_propellant_id(oxidizer_propellant_id)
        .with_specific_impulse_sea_level(specific_impulse_sea_level)
        .with_integral_propellant_mass(integral_propellant_mass)
        .build()
    }

    pub fn component_id(&self) -> Uuid {
        self.component_id
    }

    pub fn fuel_propellant_id(&self) -> Uuid {
        self.fuel_propellant_id
    }

    pub fn oxidizer_propellant_id(&self) -> Option<Uuid> {
        self.oxidizer_propellant_id
    }

    pub fn specific_impulse_vacuum(&self) -> Duration {
        self.specific_impulse_vacuum
    }

    pub fn specific_impulse_sea_level(&self) -> Option<Duration> {
        self.specific_impulse_sea_level
    }

    pub fn max_thrust(&self) -> Force {
        self.max_thrust
    }

    pub fn ignition_type(&self) -> IgnitionType {
        self.ignition_type
    }

    pub fn integral_propellant_mass(&self) -> Option<Mass> {
        self.integral_propellant_mass
    }

    pub fn effective_exhaust_velocity_vacuum(&self) -> Speed {
        Speed::new(self.specific_impulse_vacuum.value() * STANDARD_GRAVITY)
    }

    pub fn effective_exhaust_velocity_sea_level(&self) -> Option<Speed> {
        self.specific_impulse_sea_level
            .map(|isp| Speed::new(isp.value() * STANDARD_GRAVITY))
    }

    pub fn propellant_mass_flow_rate_at_max_thrust(&self) -> MassRate {
        let v_e = self.effective_exhaust_velocity_vacuum().value();
        if v_e <= 0.0 || !v_e.is_finite() {
            MassRate::new(0.0)
        } else {
            MassRate::new(self.max_thrust.value() / v_e)
        }
    }
}
