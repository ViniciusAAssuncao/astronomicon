use crate::aeroespacial::gravity::resolve_vehicle_gravitational_acceleration;
use crate::error::{RocketError, RocketResult};
use crate::orbital::nbody_field::CatalogGravityField;
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::units::{AccelerationVector, Duration, Vector3};
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::environment::EnvironmentSnapshot;

pub(super) async fn resolve_tick_gravity(
    pool: &SqlitePool,
    environment: &EnvironmentSnapshot,
    state: &VehiclePhysicalState,
    universe_epoch: Duration,
    at_epoch: Duration,
    catalog: &EphemerisCatalog,
    nbody: bool,
) -> RocketResult<AccelerationVector> {
    if !nbody {
        return resolve_vehicle_gravitational_acceleration(
            pool,
            environment,
            state,
            universe_epoch,
            at_epoch,
        )
        .await;
    }
    let mut field = CatalogGravityField::new(
        catalog,
        state.captured_total_epoch(),
        state.position().raw(),
        None,
    )?;
    field
        .acceleration_at(0.0, Vector3::zero())
        .map_err(|error| {
            RocketError::Generic(format!("N-body telemetry gravity failed: {error:?}"))
        })
}
