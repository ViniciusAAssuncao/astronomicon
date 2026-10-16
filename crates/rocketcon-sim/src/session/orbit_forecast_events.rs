use astronomicon_core::units::{Duration, GravitationalParameter, Position, VelocityVector};
use rocketcon_core::math::orbital::propagate_universal_state_vectors;

pub fn first_surface_intersection(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    interval: f64,
    radius: f64,
) -> Option<f64> {
    if !interval.is_finite() || interval <= 0.0 || !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let start = position.raw().magnitude() - radius;
    if start <= 0.0 {
        return Some(0.0);
    }
    let (end_position, end_velocity) =
        propagate_universal_state_vectors(position, velocity, mu, Duration::new(interval)).ok()?;
    let mut end = interval;
    if end_position.raw().magnitude() > radius {
        let inward_start = position.raw().dot(&velocity.raw());
        let inward_end = end_position.raw().dot(&end_velocity.raw());
        if inward_start >= 0.0 || inward_end <= 0.0 {
            return None;
        }
        let (mut low, mut high) = (0.0, interval);
        for _ in 0..42 {
            let mid = (low + high) * 0.5;
            let (r, v) =
                propagate_universal_state_vectors(position, velocity, mu, Duration::new(mid))
                    .ok()?;
            if r.raw().dot(&v.raw()) >= 0.0 {
                high = mid;
            } else {
                low = mid;
            }
        }
        end = high;
        let (periapsis, _) =
            propagate_universal_state_vectors(position, velocity, mu, Duration::new(end)).ok()?;
        if periapsis.raw().magnitude() > radius {
            return None;
        }
    }
    let (mut low, mut high) = (0.0, end);
    for _ in 0..42 {
        let mid = (low + high) * 0.5;
        let (state, _) =
            propagate_universal_state_vectors(position, velocity, mu, Duration::new(mid)).ok()?;
        if state.raw().magnitude() <= radius {
            high = mid;
        } else {
            low = mid;
        }
    }
    Some(high)
}