use crate::domain::OrbitalElements;
use crate::math::kepler::{perifocal_state_vectors, rotate_perifocal_to_system};
use crate::math::perturbation::SecularPrecessionRates;
use crate::math::rotation::angular_velocity_from_rotation_period;
use crate::units::{Angle, Duration, GravitationalParameter, Quaternion, Vector3};
use std::f64::consts::TAU;

pub fn planet_rotation_angle_at_epoch(
    rotation_period: Duration,
    prime_meridian_epoch_angle: Angle,
    total_epoch: Duration,
) -> Angle {
    let omega = angular_velocity_from_rotation_period(rotation_period);
    let angle = (prime_meridian_epoch_angle.value() + omega.value() * total_epoch.value()).rem_euclid(TAU);
    Angle::new(angle)
}

pub fn planet_rotation_axis_direction(
    orbital_plane_normal: Vector3,
    obliquity: Angle,
    solstice_reference_direction: Vector3,
) -> Vector3 {
    let n = orbital_plane_normal.normalized();
    let s = solstice_reference_direction.normalized();
    let k = n.cross(&s);
    if k.magnitude() < 1e-12 {
        n
    } else {
        n.rotate_about_axis(k.normalized(), obliquity.value()).normalized()
    }
}

pub fn solstice_reference_direction(
    elements: &OrbitalElements,
    solstice_true_anomaly: Angle,
) -> Vector3 {
    let (r_pqw, v_pqw) = perifocal_state_vectors(
        elements.semi_major_axis(),
        0.0,
        solstice_true_anomaly,
        GravitationalParameter::new(1.0),
    );
    let (r_pos, _) = rotate_perifocal_to_system(
        r_pqw,
        v_pqw,
        elements.argument_of_periapsis(),
        elements.inclination(),
        elements.longitude_of_ascending_node(),
    );
    r_pos.raw().normalized()
}

pub fn solstice_reference_direction_secular(
    elements: &OrbitalElements,
    solstice_true_anomaly: Angle,
    secular_rates: &SecularPrecessionRates,
    time_since_epoch: Duration,
) -> Vector3 {
    let (r_pqw, v_pqw) = perifocal_state_vectors(
        elements.semi_major_axis(),
        0.0,
        solstice_true_anomaly,
        GravitationalParameter::new(1.0),
    );
    let omega_t = Angle::new(
        (elements.argument_of_periapsis() + secular_rates.apsidal * time_since_epoch)
            .value()
            .rem_euclid(TAU),
    );
    let raan_t = Angle::new(
        (elements.longitude_of_ascending_node() + secular_rates.nodal * time_since_epoch)
            .value()
            .rem_euclid(TAU),
    );
    let (r_pos, _) = rotate_perifocal_to_system(
        r_pqw,
        v_pqw,
        omega_t,
        elements.inclination(),
        raan_t,
    );
    r_pos.raw().normalized()
}

pub fn resolve_planet_body_orientation(
    spin_axis_direction: Vector3,
    rotation_angle: Angle,
) -> Quaternion {
    let axis = if spin_axis_direction.magnitude() < 1e-12 {
        Vector3::new(0.0, 0.0, 1.0)
    } else {
        spin_axis_direction.normalized()
    };
    let q_align = Quaternion::from_rotation_between(Vector3::new(0.0, 0.0, 1.0), axis);
    let q_spin = Quaternion::from_axis_angle(axis, rotation_angle);
    (q_spin * q_align).normalized()
}