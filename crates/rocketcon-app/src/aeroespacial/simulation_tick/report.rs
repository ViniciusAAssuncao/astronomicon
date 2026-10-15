use crate::aeroespacial::aerodynamics::AerodynamicDiagnostic;
use crate::error::{RocketError, RocketResult};
use crate::power::thermal::VehicleThermalBudget;
use astronomicon_app::ephemeris::CelestialState;
use astronomicon_core::units::{
    Acceleration, AccelerationVector, Duration, HeatFlux, Length, Position, Pressure,
    VelocityVector,
};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::collision::SurfaceContactState;
use rocketcon_core::math::power_budget::VehiclePowerBudget;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub(super) struct ReferenceBodyTelemetry {
    pub position: Position,
    velocity: VelocityVector,
    radius_m: f64,
}

impl ReferenceBodyTelemetry {
    pub(super) fn resolve(
        states: &HashMap<Uuid, CelestialState>,
        reference_id: Uuid,
        radius: Option<Length>,
    ) -> RocketResult<Self> {
        let state = states.get(&reference_id).ok_or_else(|| {
            RocketError::Generic("reference body position unavailable after propagation".into())
        })?;
        Ok(Self {
            position: state.position,
            velocity: state.velocity,
            radius_m: radius.map_or(0.0, |radius| radius.value()),
        })
    }

    pub(super) fn position_m(self) -> [f64; 3] {
        let position = self.position.raw();
        [position.0, position.1, position.2]
    }

    pub(super) fn velocity_m_s(self) -> [f64; 3] {
        let velocity = self.velocity.raw();
        [velocity.0, velocity.1, velocity.2]
    }

    pub(super) fn radius_m(self) -> f64 {
        self.radius_m
    }
}

pub(super) fn reconcile_thermal_budget(
    mut budget: VehicleThermalBudget,
    stagnation_heat_flux: Option<HeatFlux>,
) -> VehicleThermalBudget {
    if let Some(flux) = stagnation_heat_flux {
        budget.stagnation_heat_flux = flux;
    }
    budget
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SoiTransitionReport {
    pub elapsed_seconds: f64,
    pub entering: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleTickReport {
    pub physical_state: VehiclePhysicalState,
    pub aerodynamics: Option<AerodynamicDiagnostic>,
    pub gravitational_acceleration: AccelerationVector,
    pub surface_contact: SurfaceContactState,
    pub power_budget: VehiclePowerBudget,
    pub thermal_budget: VehicleThermalBudget,
    pub axial_g_load: f64,
    pub lateral_g_load: f64,
    pub total_g_load: f64,
    pub reference_body_position_m: [f64; 3],
    pub reference_body_velocity_m_s: [f64; 3],
    pub reference_body_radius_m: f64,
    pub main_engine_loads: Vec<(Uuid, f64)>,
    pub tank_stored_kg: Vec<(Uuid, f64)>,
    pub soi_transitions: Vec<SoiTransitionReport>,
}

impl VehicleTickReport {
    pub fn new(
        physical_state: VehiclePhysicalState,
        aerodynamics: Option<AerodynamicDiagnostic>,
        gravitational_acceleration: AccelerationVector,
        surface_contact: SurfaceContactState,
        power_budget: VehiclePowerBudget,
        thermal_budget: VehicleThermalBudget,
        axial_g_load: f64,
        lateral_g_load: f64,
        total_g_load: f64,
        reference_body_position_m: [f64; 3],
        reference_body_velocity_m_s: [f64; 3],
        reference_body_radius_m: f64,
        main_engine_loads: Vec<(Uuid, f64)>,
        tank_stored_kg: Vec<(Uuid, f64)>,
        soi_transitions: Vec<SoiTransitionReport>,
    ) -> Self {
        Self {
            physical_state,
            aerodynamics,
            gravitational_acceleration,
            surface_contact,
            power_budget,
            thermal_budget,
            axial_g_load,
            lateral_g_load,
            total_g_load,
            reference_body_position_m,
            reference_body_velocity_m_s,
            reference_body_radius_m,
            main_engine_loads,
            tank_stored_kg,
            soi_transitions,
        }
    }

    pub fn physical_state(&self) -> &VehiclePhysicalState {
        &self.physical_state
    }

    pub fn aerodynamics(&self) -> Option<&AerodynamicDiagnostic> {
        self.aerodynamics.as_ref()
    }

    pub fn gravitational_acceleration(&self) -> AccelerationVector {
        self.gravitational_acceleration
    }

    pub fn gravity_magnitude(&self) -> Acceleration {
        self.gravitational_acceleration.magnitude()
    }

    pub fn surface_contact(&self) -> &SurfaceContactState {
        &self.surface_contact
    }

    pub fn has_contact(&self) -> bool {
        self.surface_contact.has_contact()
    }

    pub fn power_budget(&self) -> &VehiclePowerBudget {
        &self.power_budget
    }

    pub fn thermal_budget(&self) -> &VehicleThermalBudget {
        &self.thermal_budget
    }

    pub fn mach_number(&self) -> Option<f64> {
        self.aerodynamics.map(|a| a.mach_number)
    }

    pub fn dynamic_pressure(&self) -> Option<Pressure> {
        self.aerodynamics.map(|a| a.dynamic_pressure)
    }

    pub fn axial_g_load(&self) -> f64 {
        self.axial_g_load
    }

    pub fn lateral_g_load(&self) -> f64 {
        self.lateral_g_load
    }

    pub fn total_g_load(&self) -> f64 {
        self.total_g_load
    }

    pub fn max_dynamic_pressure(&self) -> Option<Pressure> {
        self.physical_state.max_dynamic_pressure()
    }

    pub fn max_q(&self) -> Option<Pressure> {
        self.physical_state.max_q()
    }

    pub fn max_q_epoch(&self) -> Option<Duration> {
        self.physical_state.max_q_epoch()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::{Luminosity, Temperature, Vector3};

    #[test]
    fn reference_body_telemetry_uses_selected_final_ephemeris_state() {
        let reference_id = Uuid::from_u128(1);
        let other_id = Uuid::from_u128(2);
        let mut states = HashMap::new();
        states.insert(
            other_id,
            CelestialState {
                position: Position::from_raw(Vector3::new(900.0, 800.0, 700.0)),
                velocity: VelocityVector::from_raw(Vector3::zero()),
                epoch: Duration::new(4.0),
            },
        );
        states.insert(
            reference_id,
            CelestialState {
                position: Position::from_raw(Vector3::new(1.0, -2.0, 3.0)),
                velocity: VelocityVector::from_raw(Vector3::new(-4.0, 5.0, 6.0)),
                epoch: Duration::new(4.0),
            },
        );

        let telemetry =
            ReferenceBodyTelemetry::resolve(&states, reference_id, Some(Length::new(700.0)))
                .unwrap();
        assert_eq!(telemetry.position_m(), [1.0, -2.0, 3.0]);
        assert_eq!(telemetry.velocity_m_s(), [-4.0, 5.0, 6.0]);
        assert_eq!(telemetry.radius_m(), 700.0);
        assert_eq!(
            ReferenceBodyTelemetry::resolve(&states, reference_id, None)
                .unwrap()
                .radius_m(),
            0.0
        );
        assert!(ReferenceBodyTelemetry::resolve(&states, Uuid::from_u128(3), None).is_err());
    }

    #[test]
    fn thermal_report_reconciliation_replaces_only_aerodynamic_stagnation_flux() {
        let original = VehicleThermalBudget::new_simple(
            Luminosity::new(100.0),
            4.0,
            Temperature::new(300.0),
            Temperature::new(350.0),
        );
        assert_eq!(reconcile_thermal_budget(original, None), original);

        let adjusted = reconcile_thermal_budget(original, Some(HeatFlux::new(250.0)));
        assert_eq!(adjusted.stagnation_heat_flux, HeatFlux::new(250.0));
        assert_eq!(
            adjusted.total_internal_heat_generation,
            original.total_internal_heat_generation
        );
        assert_eq!(adjusted.total_heat_input, original.total_heat_input);
        assert_eq!(
            adjusted.equilibrium_temperature,
            original.equilibrium_temperature
        );
        assert_eq!(adjusted.is_overheating, original.is_overheating);
    }
}
