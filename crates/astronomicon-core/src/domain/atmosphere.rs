use crate::chemistry::molar_mass::{
    mean_mass_attenuation_coefficient, mean_molar_mass, mean_specific_heat_capacity,
};
use crate::chemistry::viscosity::mean_dynamic_viscosity;
use crate::domain::gas_component::GasComponent;
use crate::domain::validation::{
    validate_composition, validate_finite, validate_finite_and_non_negative, validate_unit_interval,
};
use crate::error::DomainResult;
use crate::units::constants::{ATMOSPHERE_COMPOSITION_MAX_PERCENT_OVERAGE, UNIVERSAL_GAS_CONSTANT};
use crate::units::{
    Acceleration, Density, DynamicViscosity, Length, MassAttenuationCoefficient, MolarMass,
    Pressure, Temperature, TemperatureGradient,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AtmosphereBuilder {
    id: Uuid,
    planet_id: Uuid,
    surface_pressure: Pressure,
    greenhouse_effect: Temperature,
    lapse_rate: TemperatureGradient,
    composition: Vec<GasComponent>,
    surface_humidity: Option<f64>,
    cloud_coverage_fraction: Option<f64>,
    cloud_condensation_nuclei_factor: Option<f64>,
}

impl AtmosphereBuilder {
    pub fn new(
        id: Uuid,
        planet_id: Uuid,
        surface_pressure: Pressure,
        greenhouse_effect: Temperature,
        lapse_rate: TemperatureGradient,
    ) -> Self {
        Self {
            id,
            planet_id,
            surface_pressure,
            greenhouse_effect,
            lapse_rate,
            composition: Vec::new(),
            surface_humidity: None,
            cloud_coverage_fraction: None,
            cloud_condensation_nuclei_factor: None,
        }
    }

    pub fn with_composition(mut self, composition: Vec<GasComponent>) -> Self {
        self.composition = composition;
        self
    }

    pub fn with_gas_component(mut self, component: GasComponent) -> Self {
        self.composition.push(component);
        self
    }

    pub fn with_surface_humidity(mut self, surface_humidity: impl Into<Option<f64>>) -> Self {
        self.surface_humidity = surface_humidity.into();
        self
    }

    pub fn with_cloud_coverage_fraction(
        mut self,
        cloud_coverage_fraction: impl Into<Option<f64>>,
    ) -> Self {
        self.cloud_coverage_fraction = cloud_coverage_fraction.into();
        self
    }

    pub fn with_cloud_condensation_nuclei_factor(
        mut self,
        cloud_condensation_nuclei_factor: impl Into<Option<f64>>,
    ) -> Self {
        self.cloud_condensation_nuclei_factor = cloud_condensation_nuclei_factor.into();
        self
    }

    pub fn build(self) -> DomainResult<Atmosphere> {
        validate_finite_and_non_negative(self.surface_pressure.value(), "surface_pressure")?;
        validate_finite(self.greenhouse_effect.value(), "greenhouse_effect")?;
        validate_finite(self.lapse_rate.value(), "lapse_rate")?;

        if let Some(sh) = self.surface_humidity {
            validate_unit_interval(sh, "surface_humidity")?;
        }

        if let Some(cc) = self.cloud_coverage_fraction {
            validate_unit_interval(cc, "cloud_coverage_fraction")?;
        }

        if let Some(ccn) = self.cloud_condensation_nuclei_factor {
            validate_unit_interval(ccn, "cloud_condensation_nuclei_factor")?;
        }

        validate_composition(
            &self.composition,
            |c| c.percentage(),
            |c| c.formula(),
            "composition",
            "formula",
            ATMOSPHERE_COMPOSITION_MAX_PERCENT_OVERAGE,
        )?;

        Ok(Atmosphere {
            id: self.id,
            planet_id: self.planet_id,
            surface_pressure: self.surface_pressure,
            greenhouse_effect: self.greenhouse_effect,
            lapse_rate: self.lapse_rate,
            composition: self.composition,
            surface_humidity: self.surface_humidity,
            cloud_coverage_fraction: self.cloud_coverage_fraction,
            cloud_condensation_nuclei_factor: self.cloud_condensation_nuclei_factor,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Atmosphere {
    id: Uuid,
    planet_id: Uuid,
    surface_pressure: Pressure,
    greenhouse_effect: Temperature,
    lapse_rate: TemperatureGradient,
    composition: Vec<GasComponent>,
    surface_humidity: Option<f64>,
    cloud_coverage_fraction: Option<f64>,
    cloud_condensation_nuclei_factor: Option<f64>,
}

impl Atmosphere {
    pub fn builder(
        id: Uuid,
        planet_id: Uuid,
        surface_pressure: Pressure,
        greenhouse_effect: Temperature,
        lapse_rate: TemperatureGradient,
    ) -> AtmosphereBuilder {
        AtmosphereBuilder::new(
            id,
            planet_id,
            surface_pressure,
            greenhouse_effect,
            lapse_rate,
        )
    }

    pub fn new(
        id: Uuid,
        planet_id: Uuid,
        surface_pressure: Pressure,
        greenhouse_effect: Temperature,
        lapse_rate: TemperatureGradient,
        composition: Vec<GasComponent>,
        surface_humidity: Option<f64>,
        cloud_coverage_fraction: Option<f64>,
        cloud_condensation_nuclei_factor: Option<f64>,
    ) -> DomainResult<Self> {
        Self::builder(
            id,
            planet_id,
            surface_pressure,
            greenhouse_effect,
            lapse_rate,
        )
        .with_composition(composition)
        .with_surface_humidity(surface_humidity)
        .with_cloud_coverage_fraction(cloud_coverage_fraction)
        .with_cloud_condensation_nuclei_factor(cloud_condensation_nuclei_factor)
        .build()
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn planet_id(&self) -> Uuid {
        self.planet_id
    }

    pub fn surface_pressure(&self) -> Pressure {
        self.surface_pressure
    }

    pub fn greenhouse_effect(&self) -> Temperature {
        self.greenhouse_effect
    }

    pub fn lapse_rate(&self) -> TemperatureGradient {
        self.lapse_rate
    }

    pub fn composition(&self) -> &[GasComponent] {
        &self.composition
    }

    pub fn surface_humidity(&self) -> Option<f64> {
        self.surface_humidity
    }

    pub fn cloud_coverage_fraction(&self) -> Option<f64> {
        self.cloud_coverage_fraction
    }

    pub fn cloud_condensation_nuclei_factor(&self) -> Option<f64> {
        self.cloud_condensation_nuclei_factor
    }

    pub fn mean_molar_mass(&self) -> DomainResult<MolarMass> {
        let mapped: Vec<(String, f64)> = self
            .composition
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        mean_molar_mass(&mapped)
    }

    pub fn mean_specific_heat_capacity(&self) -> DomainResult<f64> {
        let mapped: Vec<(String, f64)> = self
            .composition
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        mean_specific_heat_capacity(&mapped)
    }

    pub fn mean_mass_attenuation_coefficient(&self) -> DomainResult<MassAttenuationCoefficient> {
        let mapped: Vec<(String, f64)> = self
            .composition
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        mean_mass_attenuation_coefficient(&mapped)
    }

    pub fn mean_dynamic_viscosity(
        &self,
        temperature: Temperature,
    ) -> DomainResult<DynamicViscosity> {
        let mapped: Vec<(String, f64)> = self
            .composition
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        mean_dynamic_viscosity(&mapped, temperature)
    }

    pub fn mass_column(&self, gravity: Acceleration) -> f64 {
        crate::math::radiation::atmospheric_mass_column(self.surface_pressure, gravity)
    }

    pub fn radiation_transmission(&self, gravity: Acceleration) -> DomainResult<f64> {
        let mu = self.mean_mass_attenuation_coefficient()?;
        let mass_col = self.mass_column(gravity);
        Ok(crate::math::radiation::atmospheric_transmission(
            mass_col, mu,
        ))
    }

    pub fn column_heat_capacity(&self, gravity: Acceleration) -> DomainResult<f64> {
        let cp_gas = self.mean_specific_heat_capacity()?;
        Ok(crate::math::climate::atmospheric_column_heat_capacity(
            self.surface_pressure,
            gravity,
            cp_gas,
        ))
    }

    pub fn density_at_surface(&self, surface_temperature: Temperature) -> DomainResult<Density> {
        if surface_temperature.value() <= 0.0 {
            return Ok(Density::new(0.0));
        }
        let molar_mass = self.mean_molar_mass()?;
        let rho = (self.surface_pressure.value() * molar_mass.value())
            / (UNIVERSAL_GAS_CONSTANT * surface_temperature.value());
        Ok(Density::new(rho))
    }

    pub fn scale_height(
        &self,
        gravity: Acceleration,
        surface_temperature: Temperature,
    ) -> DomainResult<Length> {
        let molar_mass = self.mean_molar_mass()?;
        let denom = molar_mass.value() * gravity.value();
        if denom <= 0.0 {
            return Ok(Length::new(0.0));
        }
        let h = (UNIVERSAL_GAS_CONSTANT * surface_temperature.value()) / denom;
        Ok(Length::new(h))
    }

    pub fn pressure_at_altitude(&self, altitude: Length, scale_height: Length) -> Pressure {
        if scale_height.value() <= 0.0 {
            if altitude.value() <= 0.0 {
                return self.surface_pressure;
            }
            return Pressure::new(0.0);
        }
        let exponent = -altitude.value() / scale_height.value();
        Pressure::new(self.surface_pressure.value() * exponent.exp())
    }
}