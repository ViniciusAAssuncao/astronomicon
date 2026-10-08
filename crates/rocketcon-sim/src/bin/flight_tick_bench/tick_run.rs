use super::{DT, sql_trace};
use astronomicon_core::units::Duration;
use rocketcon_core::domain::VehicleControlInput;
use rocketcon_db::tick_transaction::TickTransactionSession;
use std::error::Error;
use uuid::Uuid;

pub async fn run_tick(
    session: &TickTransactionSession,
    vehicle_id: Uuid,
    epoch: Duration,
    input: &VehicleControlInput,
    profiling: bool,
    trace_sql: bool,
) -> Result<
    (
        rocketcon_app::aeroespacial::VehicleTickReport,
        Option<rocketcon_app::aeroespacial::TickProfile>,
    ),
    Box<dyn Error>,
> {
    if trace_sql {
        let transaction = session.begin().await?;
        let mut connection = transaction.pool().acquire().await?;
        sql_trace::install(&mut connection).await?;
        drop(connection);
        let output =
            if profiling {
                let (report, profile) =
                rocketcon_app::aeroespacial::advance_vehicle_simulation_profiled_with_transaction(
                    &transaction, vehicle_id, Duration::new(DT), epoch, input,
                ).await?;
                (report, Some(profile))
            } else {
                let report =
                    rocketcon_app::aeroespacial::advance_vehicle_simulation_with_transaction(
                        &transaction,
                        vehicle_id,
                        Duration::new(DT),
                        epoch,
                        input,
                    )
                    .await?;
                (report, None)
            };
        transaction.commit().await?;
        return Ok(output);
    }
    if profiling {
        let (report, profile) =
            rocketcon_app::aeroespacial::advance_vehicle_simulation_profiled_in_session(
                session,
                vehicle_id,
                Duration::new(DT),
                epoch,
                input,
            )
            .await?;
        Ok((report, Some(profile)))
    } else {
        let report = rocketcon_app::aeroespacial::advance_vehicle_simulation_in_session(
            session,
            vehicle_id,
            Duration::new(DT),
            epoch,
            input,
        )
        .await?;
        Ok((report, None))
    }
}
