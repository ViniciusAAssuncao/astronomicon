use super::{TickProfile, VehicleTickReport, advance_vehicle_simulation_inner};
use crate::error::RocketResult;
use astronomicon_core::units::Duration;
use astronomicon_db::SqlitePool;
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_db::tick_transaction::{TickTransactionPool, TickTransactionSession};
use uuid::Uuid;

pub async fn advance_vehicle_simulation(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
) -> RocketResult<VehicleTickReport> {
    advance_vehicle_simulation_transactional(
        pool,
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        None,
    )
    .await
}

pub async fn advance_vehicle_simulation_profiled(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
) -> RocketResult<(VehicleTickReport, TickProfile)> {
    let mut profile = TickProfile::new();
    let report = advance_vehicle_simulation_transactional(
        pool,
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        Some(&mut profile),
    )
    .await?;
    Ok((report, profile))
}

pub async fn advance_vehicle_simulation_in_session(
    session: &TickTransactionSession,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
) -> RocketResult<VehicleTickReport> {
    let transaction = session.begin().await?;
    run_transaction(
        transaction,
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        None,
    )
    .await
}

pub async fn advance_vehicle_simulation_profiled_in_session(
    session: &TickTransactionSession,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
) -> RocketResult<(VehicleTickReport, TickProfile)> {
    let transaction = session.begin().await?;
    let mut profile = TickProfile::new();
    let report = run_transaction(
        transaction,
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        Some(&mut profile),
    )
    .await?;
    Ok((report, profile))
}

pub async fn advance_vehicle_simulation_with_transaction(
    transaction: &TickTransactionPool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
) -> RocketResult<VehicleTickReport> {
    advance_vehicle_simulation_inner(
        transaction.pool(),
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        None,
    )
    .await
}

pub async fn advance_vehicle_simulation_profiled_with_transaction(
    transaction: &TickTransactionPool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
) -> RocketResult<(VehicleTickReport, TickProfile)> {
    let mut profile = TickProfile::new();
    let report = advance_vehicle_simulation_inner(
        transaction.pool(),
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        Some(&mut profile),
    )
    .await?;
    Ok((report, profile))
}

async fn advance_vehicle_simulation_transactional(
    source_pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
    profile: Option<&mut TickProfile>,
) -> RocketResult<VehicleTickReport> {
    let transaction = TickTransactionPool::begin(source_pool).await?;
    run_transaction(
        transaction,
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        profile,
    )
    .await
}

async fn run_transaction(
    transaction: TickTransactionPool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    control_input: &VehicleControlInput,
    profile: Option<&mut TickProfile>,
) -> RocketResult<VehicleTickReport> {
    let result = advance_vehicle_simulation_inner(
        transaction.pool(),
        vehicle_id,
        dt,
        universe_epoch,
        control_input,
        profile,
    )
    .await;
    match result {
        Ok(report) => {
            transaction.commit().await?;
            Ok(report)
        }
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}
