use crate::aeroespacial::aerodynamics::{
    AerodynamicDiagnostic, resolve_vehicle_aerodynamics_in_existing_transaction,
};
use crate::error::RocketResult;
use astronomicon_app::climate::temperature::ClimateBodyInputs;
use astronomicon_core::units::{Duration, Position};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::{ComponentRecord, VehicleComponentEntry, VehiclePhysicalState};
use rocketcon_core::environment::EnvironmentSnapshot;

pub(super) struct AerodynamicSampler<'a> {
    pub pool: &'a SqlitePool,
    pub components: &'a [(VehicleComponentEntry, ComponentRecord)],
    pub active_stages: &'a [u32],
    pub universe_epoch: Duration,
    pub climate_body: Option<&'a ClimateBodyInputs>,
}

impl AerodynamicSampler<'_> {
    pub async fn sample(
        &self,
        environment: &EnvironmentSnapshot,
        physical_state: &VehiclePhysicalState,
        reference_position: Position,
        at_epoch: Duration,
    ) -> RocketResult<Option<AerodynamicDiagnostic>> {
        if environment.planet.is_none() {
            return Ok(None);
        }
        Ok(resolve_vehicle_aerodynamics_in_existing_transaction(
            self.pool,
            physical_state,
            physical_state.reference_body_id(),
            reference_position.raw(),
            self.components,
            self.active_stages,
            self.universe_epoch,
            at_epoch,
            self.climate_body,
            Some(environment),
        )
        .await?)
    }
}
