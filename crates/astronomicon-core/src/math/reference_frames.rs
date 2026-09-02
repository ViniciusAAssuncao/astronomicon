use crate::units::{AngularVelocityVector, Position, Quaternion, Vector3, Velocity};

pub use crate::math::shape::geodetic_altitude_and_normal;

pub fn inertial_to_body_fixed_position(
    planet_position: Position,
    planet_orientation: Quaternion,
    inertial_position: Position,
) -> Position {
    let r_rel = inertial_position.raw() - planet_position.raw();
    let r_body = planet_orientation.inverse().rotate_vector(r_rel);
    Position::from_raw(r_body)
}

pub fn body_fixed_to_inertial_position(
    planet_position: Position,
    planet_orientation: Quaternion,
    body_fixed_position: Position,
) -> Position {
    let r_rel = planet_orientation.rotate_vector(body_fixed_position.raw());
    Position::from_raw(planet_position.raw() + r_rel)
}

pub fn inertial_to_body_fixed_velocity(
    planet_position: Position,
    planet_velocity: Velocity,
    planet_orientation: Quaternion,
    planet_angular_velocity: AngularVelocityVector,
    inertial_position: Position,
    inertial_velocity: Velocity,
) -> Velocity {
    let r_rel = inertial_position.raw() - planet_position.raw();
    let v_rel = inertial_velocity.raw() - planet_velocity.raw();
    let omega = planet_angular_velocity.raw();
    let v_eff = v_rel - omega.cross(&r_rel);
    let v_body = planet_orientation.inverse().rotate_vector(v_eff);
    Velocity::from_raw(v_body)
}

pub fn body_fixed_to_inertial_velocity(
    _planet_position: Position,
    planet_velocity: Velocity,
    planet_orientation: Quaternion,
    planet_angular_velocity: AngularVelocityVector,
    body_fixed_position: Position,
    body_fixed_velocity: Velocity,
) -> Velocity {
    let r_rel = planet_orientation.rotate_vector(body_fixed_position.raw());
    let v_rot = planet_orientation.rotate_vector(body_fixed_velocity.raw());
    let omega = planet_angular_velocity.raw();
    let v_inertial = planet_velocity.raw() + v_rot + omega.cross(&r_rel);
    Velocity::from_raw(v_inertial)
}

pub fn topocentric_basis(
    geodetic_normal: Vector3,
    spin_axis_direction: Vector3,
) -> (Vector3, Vector3, Vector3) {
    let up = if geodetic_normal.magnitude() < 1e-12 {
        Vector3::new(0.0, 0.0, 1.0)
    } else {
        geodetic_normal.normalized()
    };
    let spin = if spin_axis_direction.magnitude() < 1e-12 {
        Vector3::new(0.0, 0.0, 1.0)
    } else {
        spin_axis_direction.normalized()
    };
    let mut east = spin.cross(&up);
    if east.magnitude() < 1e-12 {
        east = up.any_perpendicular();
    } else {
        east = east.normalized();
    }
    let north = up.cross(&east).normalized();
    (east, north, up)
}