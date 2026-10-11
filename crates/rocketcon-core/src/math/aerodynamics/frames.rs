use super::types::AerodynamicAngles;
use astronomicon_core::units::{
    Angle,
    AngularVelocityVector,
    Quaternion,
    Vector3,
    VelocityVector,
};

pub fn relative_velocity_body_frame(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3,
) -> Vector3 {
    vehicle_orientation.inverse().rotate_vector(relative_airspeed_world)
}

pub fn decompose_relative_velocity(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3,
) -> (Vector3, Vector3) {
    let v_body = relative_velocity_body_frame(vehicle_orientation, relative_airspeed_world);
    let axial = Vector3::new(0.0, 0.0, v_body.2);
    let lateral = Vector3::new(v_body.0, v_body.1, 0.0);
    (axial, lateral)
}

pub fn decompose_relative_velocity_world(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3,
) -> (Vector3, Vector3) {
    let (axial_body, lateral_body) = decompose_relative_velocity(
        vehicle_orientation,
        relative_airspeed_world,
    );
    (
        vehicle_orientation.rotate_vector(axial_body),
        vehicle_orientation.rotate_vector(lateral_body),
    )
}

pub fn compute_aerodynamic_angles(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3,
) -> AerodynamicAngles {
    let v_body = relative_velocity_body_frame(vehicle_orientation, relative_airspeed_world);
    let v_mag = v_body.magnitude();

    if v_mag < 1e-6 || !v_mag.is_finite() {
        return AerodynamicAngles::zero();
    }

    let vx = v_body.0;
    let vy = v_body.1;
    let vz = v_body.2;

    let v_lat = (vx * vx + vy * vy).sqrt();
    let total_aoa = v_lat.atan2(vz);
    let aoa = vy.atan2(vz);
    let sideslip = (vx / v_mag).clamp(-1.0, 1.0).asin();

    AerodynamicAngles::new(Angle::new(aoa), Angle::new(sideslip), Angle::new(total_aoa))
}

pub fn aerodynamic_angles(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3,
) -> AerodynamicAngles {
    compute_aerodynamic_angles(vehicle_orientation, relative_airspeed_world)
}

pub fn local_atmospheric_relative_velocity(
    vehicle_velocity_inertial: VelocityVector,
    planet_angular_velocity_inertial: AngularVelocityVector,
    vehicle_position_relative_to_planet_center_inertial: Vector3,
    wind_velocity_topocentric: Vector3,
    topocentric_east: Vector3,
    topocentric_north: Vector3,
    topocentric_up: Vector3,
) -> VelocityVector {
    let omega = planet_angular_velocity_inertial.raw();
    let r_rel = vehicle_position_relative_to_planet_center_inertial;
    let v_corot = omega.cross(&r_rel);

    let v_wind_inertial =
        topocentric_east * wind_velocity_topocentric.0 +
        topocentric_north * wind_velocity_topocentric.1 +
        topocentric_up * wind_velocity_topocentric.2;

    let v_atm_inertial = v_corot + v_wind_inertial;
    let v_rel = vehicle_velocity_inertial.raw() - v_atm_inertial;

    VelocityVector::from_raw(v_rel)
}