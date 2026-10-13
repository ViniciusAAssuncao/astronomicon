use astronomicon_core::domain::Planet;
use astronomicon_core::units::{Duration, GravitationalParameter, Mass, Position, VelocityVector};
use rocketcon_core::math::orbital::propagate_universal_state_vectors;
use rocketcon_core::math::orbital::sphere_of_influence::laplace_sphere_of_influence_radius;

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitSoiPreview {
    pub radius_m: f64,
    pub outside: bool,
    pub exit: Option<super::OrbitApsis>,
}

pub fn soi_radius(planet: &Planet, parent_mass: Mass) -> Option<f64> {
    let elements = planet.orbital_elements()?;
    let radius =
        laplace_sphere_of_influence_radius(elements.semi_major_axis(), planet.mass(), parent_mass)
            .value();
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
    if !interval_seconds.is_finite()
        || interval_seconds <= 0.0
        || !radius_m.is_finite()
        || radius_m <= 0.0
        || outside_soi(position, radius_m)
    {
        return None;
    }
    let endpoint =
        propagate_universal_state_vectors(position, velocity, mu, Duration::new(interval_seconds))
            .ok()?
            .0;
    if !outside_soi(endpoint, radius_m) {
        return None;
    }
    let (mut lo, mut hi) = (0.0, interval_seconds);
    for _ in 0..26 {
        let mid = (lo + hi) * 0.5;
        let point = propagate_universal_state_vectors(position, velocity, mu, Duration::new(mid))
            .ok()?
            .0;
        if outside_soi(point, radius_m) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let point = propagate_universal_state_vectors(position, velocity, mu, Duration::new(hi))
        .ok()?
        .0
        .raw();
    Some(super::OrbitApsis {
        time_to_seconds: hi,
        relative_position_m: [point.0, point.1, point.2],
    })
}
