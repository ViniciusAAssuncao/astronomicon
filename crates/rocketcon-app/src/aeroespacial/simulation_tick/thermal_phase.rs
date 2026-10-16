use crate::aeroespacial::aerodynamics::AerodynamicDiagnostic;
use crate::error::RocketResult;
use crate::thermal::heat_shield_response::resolve_heat_shield_response;
use crate::thermal::network_assembly::assemble_vehicle_thermal_network_with_materials;
use crate::thermal::network_tick::{
    VehicleThermalTickReport, advance_vehicle_thermal_network_in_existing_transaction,
};
use astronomicon_core::units::{Duration, HeatFlux, Luminosity};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{ComponentRecord, VehicleComponentEntry, VehiclePhysicalState};
use rocketcon_core::environment::EnvironmentSnapshot;
use std::collections::HashMap;
use uuid::Uuid;

pub(super) struct ThermalPhase<'a> {
    pub pool: &'a SqlitePool,
    pub vehicle_id: Uuid,
    pub components: &'a [(VehicleComponentEntry, ComponentRecord)],
    pub active_stages: &'a [u32],
    pub component_waste_heats: &'a HashMap<Uuid, Luminosity>,
    pub environment: &'a EnvironmentSnapshot,
    pub physical_state: &'a VehiclePhysicalState,
    pub aerodynamics: Option<&'a AerodynamicDiagnostic>,
    pub dt: Duration,
    pub universe_epoch: Duration,
    pub at_epoch: Duration,
}

impl ThermalPhase<'_> {
    pub async fn apply_heat_shield(&self) -> RocketResult<()> {
        let raw_stag_flux = self
            .aerodynamics
            .map(|a| a.stagnation_heat_flux)
            .unwrap_or_else(|| HeatFlux::new(0.0));

        if raw_stag_flux.value() > 0.0 {
            let _shield_response = resolve_heat_shield_response(
                self.pool,
                self.components,
                self.active_stages,
                raw_stag_flux,
                self.dt,
                self.universe_epoch,
                self.at_epoch,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn advance_network(&self) -> RocketResult<VehicleThermalTickReport> {
        let (air_density, relative_airspeed, mach_number) =
            self.aerodynamics.map_or((None, None, None), |d| {
                (
                    Some(d.air_density),
                    Some(d.relative_airspeed),
                    Some(d.mach_number),
                )
            });
        let (mut thermal_network, materials) = assemble_vehicle_thermal_network_with_materials(
            self.pool,
            self.vehicle_id,
            self.components,
            self.active_stages,
            self.component_waste_heats,
            air_density,
            relative_airspeed,
            mach_number,
            None,
        )
        .await?;

        advance_vehicle_thermal_network_in_existing_transaction(
            self.pool,
            self.vehicle_id,
            &mut thermal_network,
            self.components,
            self.active_stages,
            self.environment,
            self.physical_state.position(),
            self.physical_state.orientation(),
            None,
            self.dt,
            self.universe_epoch,
            self.at_epoch,
            &materials,
        )
        .await
    }
}
