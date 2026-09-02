use crate::domain::ignition_type::IgnitionType;
use crate::domain::propellant::Propellant;
use crate::error::{ RocketDomainError, RocketDomainResult };
use astronomicon_core::domain::validation::validate_positive_finite;
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{ Angle, AngularVelocity, Duration, Force, Mass, MassRate, Speed };
use serde::{ Deserialize, Serialize };
use std::f64::consts::FRAC_PI_2;
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
    max_gimbal_deflection: Option<Angle>,
    gimbal_slew_rate: Option<AngularVelocity>,
    min_throttle_fraction: Option<f64>,
    oxidizer_to_fuel_mass_ratio: Option<f64>,
}

impl EngineSpecificationBuilder {
    pub fn new(
        component_id: Uuid,
        fuel_propellant_id: Uuid,
        specific_impulse_vacuum: Duration,
        max_thrust: Force,
        ignition_type: IgnitionType
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
            max_gimbal_deflection: None,
            gimbal_slew_rate: None,
            min_throttle_fraction: None,
            oxidizer_to_fuel_mass_ratio: None,
        }
    }

    pub fn with_oxidizer_propellant_id(
        mut self,
        oxidizer_propellant_id: impl Into<Option<Uuid>>
    ) -> Self {
        self.oxidizer_propellant_id = oxidizer_propellant_id.into();
        self
    }

    pub fn with_specific_impulse_sea_level(
        mut self,
        specific_impulse_sea_level: impl Into<Option<Duration>>
    ) -> Self {
        self.specific_impulse_sea_level = specific_impulse_sea_level.into();
        self
    }

    pub fn with_integral_propellant_mass(
        mut self,
        integral_propellant_mass: impl Into<Option<Mass>>
    ) -> Self {
        self.integral_propellant_mass = integral_propellant_mass.into();
        self
    }

    pub fn with_max_gimbal_deflection(
        mut self,
        max_gimbal_deflection: impl Into<Option<Angle>>
    ) -> Self {
        self.max_gimbal_deflection = max_gimbal_deflection.into();
        self
    }

    pub fn with_gimbal_slew_rate(
        mut self,
        gimbal_slew_rate: impl Into<Option<AngularVelocity>>
    ) -> Self {
        self.gimbal_slew_rate = gimbal_slew_rate.into();
        self
    }

    pub fn with_min_throttle_fraction(
        mut self,
        min_throttle_fraction: impl Into<Option<f64>>
    ) -> Self {
        self.min_throttle_fraction = min_throttle_fraction.into();
        self
    }

    pub fn with_oxidizer_to_fuel_mass_ratio(
        mut self,
        oxidizer_to_fuel_mass_ratio: impl Into<Option<f64>>
    ) -> Self {
        self.oxidizer_to_fuel_mass_ratio = oxidizer_to_fuel_mass_ratio.into();
        self
    }

    pub fn build(self) -> RocketDomainResult<EngineSpecification> {
        validate_positive_finite(self.specific_impulse_vacuum.value(), "specific_impulse_vacuum")?;
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
                    reason: "integral propellant mass is only allowed for SingleBurn engines".to_string(),
                });
            }
        }

        if self.oxidizer_propellant_id.is_some() != self.oxidizer_to_fuel_mass_ratio.is_some() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "oxidizer_to_fuel_mass_ratio".to_string(),
                reason: "must be present if and only if oxidizer_propellant_id is present".to_string(),
            });
        }

        if let Some(of) = self.oxidizer_to_fuel_mass_ratio {
            validate_positive_finite(of, "oxidizer_to_fuel_mass_ratio")?;
        }

        if self.max_gimbal_deflection.is_some() != self.gimbal_slew_rate.is_some() {
            return Err(RocketDomainError::InvalidInvariant {
                field: "gimbal".to_string(),
                reason: "max_gimbal_deflection and gimbal_slew_rate must both be present or both absent".to_string(),
            });
        }

        if let Some(deflection) = self.max_gimbal_deflection {
            validate_positive_finite(deflection.value(), "max_gimbal_deflection")?;
            if deflection.value() >= FRAC_PI_2 {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "max_gimbal_deflection".to_string(),
                    reason: "must be less than pi / 2 radians".to_string(),
                });
            }
        }

        if let Some(rate) = self.gimbal_slew_rate {
            validate_positive_finite(rate.value(), "gimbal_slew_rate")?;
        }

        if let Some(throttle) = self.min_throttle_fraction {
            validate_positive_finite(throttle, "min_throttle_fraction")?;
            if throttle > 1.0 {
                return Err(RocketDomainError::InvalidInvariant {
                    field: "min_throttle_fraction".to_string(),
                    reason: "must be less than or equal to 1.0".to_string(),
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
            max_gimbal_deflection: self.max_gimbal_deflection,
            gimbal_slew_rate: self.gimbal_slew_rate,
            min_throttle_fraction: self.min_throttle_fraction,
            oxidizer_to_fuel_mass_ratio: self.oxidizer_to_fuel_mass_ratio,
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
    max_gimbal_deflection: Option<Angle>,
    gimbal_slew_rate: Option<AngularVelocity>,
    min_throttle_fraction: Option<f64>,
    oxidizer_to_fuel_mass_ratio: Option<f64>,
}

impl EngineSpecification {
    pub fn builder(
        component_id: Uuid,
        fuel_propellant_id: Uuid,
        specific_impulse_vacuum: Duration,
        max_thrust: Force,
        ignition_type: IgnitionType
    ) -> EngineSpecificationBuilder {
        EngineSpecificationBuilder::new(
            component_id,
            fuel_propellant_id,
            specific_impulse_vacuum,
            max_thrust,
            ignition_type
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
        max_gimbal_deflection: Option<Angle>,
        gimbal_slew_rate: Option<AngularVelocity>,
        min_throttle_fraction: Option<f64>,
        oxidizer_to_fuel_mass_ratio: Option<f64>
    ) -> RocketDomainResult<Self> {
        Self::builder(
            component_id,
            fuel_propellant_id,
            specific_impulse_vacuum,
            max_thrust,
            ignition_type
        )
            .with_oxidizer_propellant_id(oxidizer_propellant_id)
            .with_specific_impulse_sea_level(specific_impulse_sea_level)
            .with_integral_propellant_mass(integral_propellant_mass)
            .with_max_gimbal_deflection(max_gimbal_deflection)
            .with_gimbal_slew_rate(gimbal_slew_rate)
            .with_min_throttle_fraction(min_throttle_fraction)
            .with_oxidizer_to_fuel_mass_ratio(oxidizer_to_fuel_mass_ratio)
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

    pub fn max_gimbal_deflection(&self) -> Option<Angle> {
        self.max_gimbal_deflection
    }

    pub fn gimbal_slew_rate(&self) -> Option<AngularVelocity> {
        self.gimbal_slew_rate
    }

    pub fn min_throttle_fraction(&self) -> Option<f64> {
        self.min_throttle_fraction
    }

    pub fn oxidizer_to_fuel_mass_ratio(&self) -> Option<f64> {
        self.oxidizer_to_fuel_mass_ratio
    }

    pub fn has_gimbal(&self) -> bool {
        self.max_gimbal_deflection.is_some()
    }

    pub fn is_throttleable(&self) -> bool {
        self.min_throttle_fraction.is_some()
    }

    pub fn requires_priming(&self, fuel: &Propellant, oxidizer: Option<&Propellant>) -> bool {
        fuel.is_cryogenic() || oxidizer.map_or(false, |o| o.is_cryogenic())
    }

    pub fn effective_exhaust_velocity_vacuum(&self) -> Speed {
        Speed::new(self.specific_impulse_vacuum.value() * STANDARD_GRAVITY)
    }

    pub fn effective_exhaust_velocity_sea_level(&self) -> Option<Speed> {
        self.specific_impulse_sea_level.map(|isp| Speed::new(isp.value() * STANDARD_GRAVITY))
    }

    pub fn propellant_mass_flow_rate_at_max_thrust(&self) -> MassRate {
        let v_e = self.effective_exhaust_velocity_vacuum().value();
        if v_e <= 0.0 || !v_e.is_finite() {
            MassRate::new(0.0)
        } else {
            MassRate::new(self.max_thrust.value() / v_e)
        }
    }

    pub fn fuel_mass_flow_rate_at_max_thrust(&self) -> MassRate {
        let total = self.propellant_mass_flow_rate_at_max_thrust().value();
        if let Some(of) = self.oxidizer_to_fuel_mass_ratio {
            MassRate::new(total / (1.0 + of))
        } else {
            MassRate::new(total)
        }
    }

    pub fn oxidizer_mass_flow_rate_at_max_thrust(&self) -> Option<MassRate> {
        self.oxidizer_to_fuel_mass_ratio.map(|of| {
            let total = self.propellant_mass_flow_rate_at_max_thrust().value();
            MassRate::new((total * of) / (1.0 + of))
        })
    }
}
