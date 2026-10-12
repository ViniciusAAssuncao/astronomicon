use astronomicon_core::domain::{OrbitalParent, Planet, Star};
use astronomicon_core::units::{Duration, GravitationalParameter, Position, VelocityVector};
use rocketcon_core::math::orbital::sphere_of_influence::laplace_sphere_of_influence_radius;
use rocketcon_core::math::orbital::propagate_universal_state_vectors;

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitSoiPreview {
    pub radius_m: f64,
    pub outside: bool,
    pub exit: Option<super::OrbitApsis>,
}

pub fn soi_radius(planet: &Planet, star: &Star) -> Option<f64> {
    if !matches!(planet.orbital_parent(), OrbitalParent::Star(id) if id == star.id()) {
        return None;
    }
    let elements = planet.orbital_elements()?;
    let radius = laplace_sphere_of_influence_radius(
        elements.semi_major_axis(), planet.mass(), star.mass()).value();
    (radius.is_finite() && radius > 0.0).then_some(radius)
}

pub fn outside_soi(position: Position, radius_m: f64) -> bool {
    position.raw().magnitude() >= radius_m
}

pub fn refine_exit(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    interval_seconds: f64,
    radius_m: f64,
) -> Option<super::OrbitApsis> {
    if !interval_seconds.is_finite() || interval_seconds <= 0.0 ||
        !radius_m.is_finite() || radius_m <= 0.0 || outside_soi(position, radius_m) {
        return None;
    }
    let endpoint = propagate_universal_state_vectors(
        position, velocity, mu, Duration::new(interval_seconds)).ok()?.0;
    if !outside_soi(endpoint, radius_m) {
        return None;
    }
    let (mut lo, mut hi) = (0.0, interval_seconds);
    for _ in 0..26 {
        let mid = (lo + hi) * 0.5;
        let point = propagate_universal_state_vectors(
            position, velocity, mu, Duration::new(mid)).ok()?.0;
        if outside_soi(point, radius_m) { hi = mid; } else { lo = mid; }
    }
    let point = propagate_universal_state_vectors(
        position, velocity, mu, Duration::new(hi)).ok()?.0.raw();
    Some(super::OrbitApsis {
        time_to_seconds: hi,
        relative_position_m: [point.0, point.1, point.2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::Vector3;

    #[test]
    fn radial_exit_is_refined_to_boundary() {
        let radius = 150_000_000.0;
        let pos = Position::from_raw(Vector3::new(radius - 400_000.0, 0.0, 0.0));
        let vel = VelocityVector::from_raw(Vector3::new(6_000.0, 0.0, 0.0));
        let mu = GravitationalParameter::new(3.8667e13);
        let exit = refine_exit(pos, vel, mu, 120.0, radius).unwrap();
        let r = exit.relative_position_m[0].hypot(exit.relative_position_m[1])
            .hypot(exit.relative_position_m[2]);
        assert!((r - radius).abs() < 0.1);
        assert!(exit.time_to_seconds > 0.0 && exit.time_to_seconds < 120.0);
    }

    #[test]
    fn inward_motion_and_outside_state_do_not_predict_exit() {
        let radius = 150_000_000.0;
        let mu = GravitationalParameter::new(3.8667e13);
        let pos = Position::from_raw(Vector3::new(radius - 400_000.0, 0.0, 0.0));
        let inward = VelocityVector::from_raw(Vector3::new(-6_000.0, 0.0, 0.0));
        assert!(refine_exit(pos, inward, mu, 120.0, radius).is_none());
        let outside = Position::from_raw(Vector3::new(radius + 2.0, 0.0, 0.0));
        assert!(outside_soi(outside, radius));
        assert!(refine_exit(outside, inward, mu, 120.0, radius).is_none());
    }

    #[test]
    fn tangential_short_arc_does_not_invent_exit() {
        let mu = GravitationalParameter::new(3.8667e13);
        let radius = 150_000_000.0;
        let pos = Position::from_raw(Vector3::new(radius - 100_000.0, 0.0, 0.0));
        let v = VelocityVector::from_raw(Vector3::new(0.0, 50.0, 0.0));
        assert!(refine_exit(pos, v, mu, 120.0, radius).is_none());
    }
}
