use astronomicon_core::units::{Duration, Pressure};
use rocketcon_core::domain::VehiclePhysicalState;

pub(crate) fn resolve_max_q(
    physical_state: &VehiclePhysicalState,
    sampled_q: Option<Pressure>,
    sampled_epoch: Duration,
) -> (Option<Pressure>, Option<Duration>) {
    match (physical_state.max_dynamic_pressure(), sampled_q) {
        (Some(prev_q), Some(current_q)) => {
            if current_q.value() > prev_q.value() {
                (Some(current_q), Some(sampled_epoch))
            } else {
                (Some(prev_q), physical_state.max_dynamic_pressure_epoch())
            }
        }
        (None, Some(current_q)) => (Some(current_q), Some(sampled_epoch)),
        (Some(prev_q), None) => (Some(prev_q), physical_state.max_dynamic_pressure_epoch()),
        (None, None) => (None, None),
    }
}