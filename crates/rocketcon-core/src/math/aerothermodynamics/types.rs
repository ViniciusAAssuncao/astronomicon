use crate::domain::trajectory_patch::LowThrustPatchData;
use crate::error::RocketDomainResult;
use crate::math::aerothermodynamics::pass_simulation::trajectory_to_chebyshev_patch;
use crate::math::orbital::types::OsculatingElements;
use astronomicon_core::units::{
    Angle, AngularVelocity, Density, Duration, GravitationalParameter, HeatFlux, Length,
    Luminosity, Mass, Position, Pressure, Speed, VelocityVector,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericModelParameters {
    pub surface_density: Density,
    pub scale_height: Length,
    pub entry_interface_radius: Length,
    pub planet_radius: Length,
    pub gravitational_parameter: GravitationalParameter,
    pub planet_rotation_rate: Option<AngularVelocity>,
}

impl AtmosphericModelParameters {
    pub fn new(
        surface_density: Density,
        scale_height: Length,
        entry_interface_radius: Length,
        planet_radius: Length,
        gravitational_parameter: GravitationalParameter,
        planet_rotation_rate: Option<AngularVelocity>,
    ) -> Self {
        Self {
            surface_density,
            scale_height,
            entry_interface_radius,
            planet_radius,
            gravitational_parameter,
            planet_rotation_rate,
        }
    }

    pub fn density_at_altitude(&self, altitude: Length) -> Density {
        let h = altitude.value();
        let h_scale = self.scale_height.value();
        let rho_0 = self.surface_density.value();
        let h_top = self.entry_interface_radius.value() - self.planet_radius.value();

        if h < 0.0 || h >= h_top || h_scale <= 0.0 || rho_0 <= 0.0 || !h.is_finite() {
            return Density::new(0.0);
        }

        let exponent = -h / h_scale;
        if exponent < -700.0 {
            Density::new(0.0)
        } else {
            Density::new(rho_0 * exponent.exp())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AerocaptureVehicleProperties {
    pub mass: Mass,
    pub reference_area_m2: f64,
    pub drag_coefficient: f64,
    pub lift_to_drag_ratio: f64,
    pub nose_radius: Length,
    pub max_allowable_heat_flux: HeatFlux,
    pub max_allowable_g_load: f64,
}

impl AerocaptureVehicleProperties {
    pub fn new(
        mass: Mass,
        reference_area_m2: f64,
        drag_coefficient: f64,
        lift_to_drag_ratio: f64,
        nose_radius: Length,
        max_allowable_heat_flux: HeatFlux,
        max_allowable_g_load: f64,
    ) -> Self {
        Self {
            mass,
            reference_area_m2,
            drag_coefficient,
            lift_to_drag_ratio,
            nose_radius,
            max_allowable_heat_flux,
            max_allowable_g_load,
        }
    }

    pub fn ballistic_coefficient(&self) -> f64 {
        let m = self.mass.value();
        let cd = self.drag_coefficient;
        let a = self.reference_area_m2;
        if cd <= 0.0 || a <= 0.0 {
            0.0
        } else {
            m / (cd * a)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericPassState {
    pub time: Duration,
    pub position: Position,
    pub velocity: VelocityVector,
    pub altitude: Length,
    pub speed: Speed,
    pub flight_path_angle: Angle,
    pub density: Density,
    pub dynamic_pressure: Pressure,
    pub stagnation_heat_flux: HeatFlux,
    pub g_load: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AerocaptureOutcome {
    Captured {
        post_pass_elements: OsculatingElements,
        exit_epoch: Duration,
    },
    Escaped {
        exit_elements: OsculatingElements,
        exit_epoch: Duration,
    },
    SurfaceImpact {
        impact_speed: Speed,
        impact_time: Duration,
    },
    ExceededThermalLimits {
        peak_heat_flux: HeatFlux,
        limit: HeatFlux,
    },
    ExceededStructuralLimits {
        peak_g_load: f64,
        limit: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AerocaptureTrajectoryResult {
    pub states: Vec<AtmosphericPassState>,
    pub outcome: AerocaptureOutcome,
    pub entry_epoch: Duration,
    pub exit_epoch: Option<Duration>,
    pub periapsis_altitude: Length,
    pub peak_dynamic_pressure: Pressure,
    pub peak_stagnation_heat_flux: HeatFlux,
    pub integrated_heat_load_j_per_m2: f64,
    pub peak_g_load: f64,
    pub post_pass_apoapsis: Option<Length>,
    pub post_pass_periapsis: Option<Length>,
    pub total_delta_v_absorbed: Speed,
}

impl AerocaptureTrajectoryResult {
    pub fn to_low_thrust_patch_data(
        &self,
        degree: usize,
    ) -> RocketDomainResult<LowThrustPatchData> {
        trajectory_to_chebyshev_patch(self, degree)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntryCorridor {
    pub overshoot_flight_path_angle: Angle,
    pub undershoot_flight_path_angle: Angle,
    pub corridor_width: Angle,
    pub overshoot_periapsis_altitude: Length,
    pub undershoot_periapsis_altitude: Length,
    pub is_viable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AerocapturePlan {
    pub target_entry_flight_path_angle: Angle,
    pub target_vacuum_periapsis_altitude: Length,
    pub target_vacuum_periapsis_radius: Length,
    pub predicted_post_pass_apoapsis: Length,
    pub predicted_post_pass_periapsis: Length,
    pub peak_stagnation_heat_flux: HeatFlux,
    pub peak_dynamic_pressure: Pressure,
    pub peak_g_load: f64,
    pub integrated_heat_load_j_per_m2: f64,
    pub total_delta_v_absorbed: Speed,
    pub atmospheric_pass_duration: Duration,
    pub corridor: EntryCorridor,
    pub trajectory: AerocaptureTrajectoryResult,
    pub is_feasible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AerothermodynamicResults {
    pub stagnation_heat_flux: HeatFlux,
    pub skin_friction_heat_flux: HeatFlux,
    pub nose_radius: Length,
    pub nose_area_m2: f64,
    pub side_area_m2: f64,
    pub total_aerodynamic_heat_power: Luminosity,
}

impl AerothermodynamicResults {
    pub fn new(
        stagnation_heat_flux: HeatFlux,
        skin_friction_heat_flux: HeatFlux,
        nose_radius: Length,
        nose_area_m2: f64,
        side_area_m2: f64,
        total_aerodynamic_heat_power: Luminosity,
    ) -> Self {
        Self {
            stagnation_heat_flux,
            skin_friction_heat_flux,
            nose_radius,
            nose_area_m2,
            side_area_m2,
            total_aerodynamic_heat_power,
        }
    }
}