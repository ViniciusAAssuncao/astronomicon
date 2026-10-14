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

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::Vector3;

    #[test]
    fn surface_crossing_is_independent_of_sampling_interval() {
        let radius = 6_400_000.0;
        let mu = GravitationalParameter::new(3.986004418e14);
        let r = Position::from_raw(Vector3::new(9_000_000.0, 0.0, 0.0));
        let v = VelocityVector::from_raw(Vector3::new(-10_000.0, 1_000.0, 0.0));
        let first = first_surface_intersection(r, v, mu, 1200.0, radius).unwrap();
        let second = first_surface_intersection(r, v, mu, 2400.0, radius).unwrap();
        assert!((first - second).abs() < 1e-7);
        let (impact, _) =
            propagate_universal_state_vectors(r, v, mu, Duration::new(first)).unwrap();
        assert!((impact.raw().magnitude() - radius).abs() < 0.02);
    }
}
