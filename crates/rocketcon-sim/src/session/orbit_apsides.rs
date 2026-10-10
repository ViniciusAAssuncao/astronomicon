use astronomicon_core::units::{Angle, Duration, GravitationalParameter, Position, VelocityVector};
use rocketcon_core::math::orbital::trajectory_prediction::time_between_true_anomalies;
use rocketcon_core::math::orbital::{propagate_universal_state_vectors, OsculatingElements};
use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct OrbitApsis {
    pub time_to_seconds: f64,
    pub relative_position_m: [f64; 3],
}

pub fn next_apsis(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    anomaly: Angle,
    horizon_seconds: f64,
) -> Option<OrbitApsis> {
    if !(1e-5..1.0).contains(&elements.eccentricity) {
        return None;
    }
    let dt = time_between_true_anomalies(elements, mu, elements.true_anomaly, anomaly)
        .ok()?
        .value();
    let boundary_tolerance = horizon_seconds * 1e-8;
    if !dt.is_finite() || dt < 0.0 || dt > horizon_seconds + boundary_tolerance {
        return None;
    }
    let dt = dt.min(horizon_seconds);
    let (apsis_position, _) =
        propagate_universal_state_vectors(position, velocity, mu, Duration::new(dt)).ok()?;
    let point = apsis_position.raw();
    if ![point.0, point.1, point.2]
        .iter()
        .all(|value| value.is_finite())
    {
        return None;
    }
    Some(OrbitApsis {
        time_to_seconds: dt,
        relative_position_m: [point.0, point.1, point.2],
    })
}

pub fn next_periapsis(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    horizon_seconds: f64,
) -> Option<OrbitApsis> {
    next_apsis(
        elements,
        position,
        velocity,
        mu,
        Angle::new(0.0),
        horizon_seconds,
    )
}

pub fn next_apoapsis(
    elements: &OsculatingElements,
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    horizon_seconds: f64,
) -> Option<OrbitApsis> {
    next_apsis(
        elements,
        position,
        velocity,
        mu,
        Angle::new(PI),
        horizon_seconds,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::Vector3;
    use rocketcon_core::math::orbital::cartesian_to_osculating_elements;

    #[test]
    fn eccentric_orbit_has_periapsis_now_and_apoapsis_half_a_period_ahead() {
        let mu = GravitationalParameter::new(3.986004418e14);
        let periapsis_radius = 7.0e6;
        let eccentricity = 0.2;
        let position = Position::from_raw(Vector3::new(periapsis_radius, 0.0, 0.0));
        let velocity = VelocityVector::from_raw(Vector3::new(
            0.0,
            (mu.value() * (1.0 + eccentricity) / periapsis_radius).sqrt(),
            0.0,
        ));
        let elements = cartesian_to_osculating_elements(position, velocity, mu).unwrap();
        let period = 2.0 * PI * (elements.semi_major_axis.value().powi(3) / mu.value()).sqrt();
        let periapsis = next_periapsis(&elements, position, velocity, mu, period).unwrap();
        let apoapsis = next_apoapsis(&elements, position, velocity, mu, period).unwrap();
        assert!(periapsis.time_to_seconds.abs() < 1e-6);
        assert!((apoapsis.time_to_seconds - period / 2.0).abs() < 1e-6);
        assert!(
            (apoapsis.relative_position_m[0] + elements.apoapsis_distance.unwrap().value()).abs()
                < 1.0
        );
        assert!(next_apoapsis(&elements, position, velocity, mu, period / 4.0).is_none());
        let circular_velocity = VelocityVector::from_raw(Vector3::new(
            0.0,
            (mu.value() / periapsis_radius).sqrt(),
            0.0,
        ));
        let circular = cartesian_to_osculating_elements(position, circular_velocity, mu).unwrap();
        assert!(next_periapsis(&circular, position, circular_velocity, mu, period).is_none());
    }

    #[test]
    fn periapsis_remains_visible_across_the_horizon_boundary() {
        let mu = GravitationalParameter::new(3.986004418e14);
        let radius = 7.0e6;
        let position = Position::from_raw(Vector3::new(radius, 0.0, 0.0));
        let velocity = VelocityVector::from_raw(Vector3::new(
            0.0,
            (mu.value() * 1.2 / radius).sqrt(),
            0.0,
        ));
        let elements = cartesian_to_osculating_elements(position, velocity, mu).unwrap();
        let period = 2.0 * PI * (elements.semi_major_axis.value().powi(3) / mu.value()).sqrt();
        for offset in [-1e-6, 0.0, 1e-6] {
            let (at_position, at_velocity) = propagate_universal_state_vectors(
                position, velocity, mu, Duration::new(offset),
            ).unwrap();
            let at_elements = cartesian_to_osculating_elements(at_position, at_velocity, mu).unwrap();
            let next = next_periapsis(
                &at_elements, at_position, at_velocity, mu, period,
            );
            assert!(next.is_some(), "periapsis marker missing at {offset}");
        }
    }
}
