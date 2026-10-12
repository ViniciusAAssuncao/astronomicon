use astronomicon_core::units::{Angle, GravitationalParameter, Length};
use rocketcon_core::math::orbital::OsculatingElements;
use rocketcon_core::math::orbital::trajectory_prediction::{
    find_soi_exit_event, time_between_true_anomalies,
};

const MAX_OPEN_HORIZON: f64 = 259_200.0;
const MAX_BOUND_HORIZON: f64 = 172_800.0;

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

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::{Position, Vector3, VelocityVector};
    use rocketcon_core::math::orbital::cartesian_to_osculating_elements;

    #[test]
    fn open_flyby_horizon_covers_more_than_one_hour() {
        let position = Position::from_raw(Vector3::new(141_473_122.607, 0.0, 0.0));
        let velocity = VelocityVector::from_raw(Vector3::new(-4_000.0, 300.0, 0.0));
        let mu = GravitationalParameter::new(3.8667e13);
        let elements = cartesian_to_osculating_elements(position, velocity, mu).unwrap();
        assert!(elements.eccentricity > 1.0);
        let horizon = preview_horizon(&elements, mu, None, Some(142_973_122.607));
        assert!(horizon > 3_600.0);
        assert!(horizon <= MAX_OPEN_HORIZON);
    }

    #[test]
    fn bound_orbit_keeps_period_but_caps_cost() {
        let position = Position::from_raw(Vector3::new(4_000_000.0, 0.0, 0.0));
        let velocity = VelocityVector::from_raw(Vector3::new(0.0, 2_800.0, 0.0));
        let mu = GravitationalParameter::new(3.8667e13);
        let elements = cartesian_to_osculating_elements(position, velocity, mu).unwrap();
        assert_eq!(preview_horizon(&elements, mu, Some(8_000.0), None), 8_000.0);
        assert_eq!(
            preview_horizon(&elements, mu, Some(300_000.0), None),
            MAX_BOUND_HORIZON
        );
    }
}
