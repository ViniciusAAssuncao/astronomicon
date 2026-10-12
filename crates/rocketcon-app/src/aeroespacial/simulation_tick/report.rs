use crate::aeroespacial::aerodynamics::AerodynamicDiagnostic;
use crate::power::thermal::VehicleThermalBudget;
use astronomicon_core::units::{Acceleration, AccelerationVector, Duration, Pressure};
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::collision::SurfaceContactState;
use rocketcon_core::math::power_budget::VehiclePowerBudget;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
