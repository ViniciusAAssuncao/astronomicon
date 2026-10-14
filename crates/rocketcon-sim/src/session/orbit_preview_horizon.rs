use astronomicon_core::units::{Angle, GravitationalParameter, Length};
use rocketcon_core::math::orbital::OsculatingElements;
use rocketcon_core::math::orbital::trajectory_prediction::{
    find_soi_exit_event, time_between_true_anomalies,
};

const MAX_OPEN_HORIZON: f64 = 259_200.0;
pub const MAX_BOUND_HORIZON: f64 = 172_800.0;

pub fn preview_horizon(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
    period_seconds: Option<f64>,
    soi_radius: Option<f64>,
) -> f64 {
    if let Some(period) = period_seconds {
        return period.clamp(600.0, MAX_BOUND_HORIZON);
    }
    let mut goal = 3_600.0_f64;
    if let Ok(dt) =
        time_between_true_anomalies(elements, mu, elements.true_anomaly, Angle::new(0.0))
    {
        if dt.value().is_finite() && dt.value() >= 0.0 {
            goal = goal.max(dt.value() * 1.3 + 1_800.0);
        }
    }
    if let Some(radius) = soi_radius {
        if let Ok(Some((_, time))) = find_soi_exit_event(elements, mu, Length::new(radius)) {
            if time.value().is_finite() && time.value() > 0.0 {
                goal = goal.max(time.value() * 1.001 + 1.0);
            }
        }
    }
    goal.clamp(3_600.0, MAX_OPEN_HORIZON)
}
