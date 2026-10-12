use super::coefficients::{ axial_drag_coefficient, normal_force_coefficient };
use super::frames::{ compute_aerodynamic_angles, relative_velocity_body_frame };
use super::geometry::{ center_of_pressure, vehicle_reference_cross_section_area };
use super::types::{ mach_regime, VehicleAerodynamics };
use crate::domain::{ ComponentRecord, VehicleComponentEntry };
use astronomicon_core::units::{
    Density,
    ForceVector,
    Pressure,
    Quaternion,
    Speed,
    TorqueVector,
    Vector3,
    VelocityVector,
};

pub fn mach_number(relative_speed: Speed, local_speed_of_sound: Speed) -> f64 {
    let v = relative_speed.value();
    let a = local_speed_of_sound.value();

    if v <= 0.0 || a <= 0.0 || !v.is_finite() || !a.is_finite() {
        0.0
    } else {
        v / a
    }
}

pub fn dynamic_pressure(density: Density, relative_speed: Speed) -> Pressure {
    let rho = density.value();
    let v = relative_speed.value();

    if rho <= 0.0 || v <= 0.0 || !rho.is_finite() || !v.is_finite() {
        return Pressure::new(0.0);
    }

    Pressure::new(0.5 * rho * v * v)
}

pub fn aerodynamic_drag_force(
    dynamic_pressure: Pressure,
    drag_coefficient: f64,
    reference_area_m2: f64,
    relative_velocity_direction_world: Vector3
) -> ForceVector {
    let q = dynamic_pressure.value();
    let cd = drag_coefficient;
    let a = reference_area_m2;

    if q <= 0.0 || cd <= 0.0 || a <= 0.0 || !q.is_finite() || !cd.is_finite() || !a.is_finite() {
        return ForceVector::zero();
    }

    let dir = relative_velocity_direction_world.normalized();
    if dir.magnitude() < 1e-12 {
        return ForceVector::zero();
    }

    let drag_magnitude = q * cd * a;
    let drag_vector = -dir * drag_magnitude;

    ForceVector::from_raw(drag_vector)
}

pub(crate) fn compute_body_aerodynamic_forces(
    v_body: Vector3,
    dynamic_pressure_times_area: f64,
    cd: f64,
    cn: f64
) -> (Vector3, Vector3, Vector3) {
    let f_axial_mag = dynamic_pressure_times_area * cd;
    let f_normal_mag = dynamic_pressure_times_area * cn.abs();

    let u_axial = if v_body.2 >= 0.0 {
        Vector3::new(0.0, 0.0, -1.0)
    } else {
        Vector3::new(0.0, 0.0, 1.0)
    };
    let f_axial_body = u_axial * f_axial_mag;

    let v_lat = Vector3::new(v_body.0, v_body.1, 0.0);
    let v_lat_mag = v_lat.magnitude();

    let f_normal_body = if v_lat_mag > 1e-9 {
        let u_lat = v_lat / v_lat_mag;
        -u_lat * f_normal_mag
    } else {
        Vector3::zero()
    };

    let f_total_body = f_axial_body + f_normal_body;
    (f_axial_body, f_normal_body, f_total_body)
}

pub fn compute_aerodynamic_forces_and_torque(
    dynamic_pressure: Pressure,
    reference_area_m2: f64,
    mach: f64,
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3,
    center_of_pressure: Vector3,
    center_of_mass: Vector3
) -> (ForceVector, TorqueVector) {
    let q = dynamic_pressure.value();
    let s = reference_area_m2;

    if q <= 0.0 || s <= 0.0 || !q.is_finite() || !s.is_finite() {
        return (ForceVector::zero(), TorqueVector::zero());
    }

    let v_body = relative_velocity_body_frame(vehicle_orientation, relative_airspeed_world);
    let v_mag = v_body.magnitude();

    if v_mag < 1e-6 || !v_mag.is_finite() {
        return (ForceVector::zero(), TorqueVector::zero());
    }

    let angles = compute_aerodynamic_angles(vehicle_orientation, relative_airspeed_world);
    let cd = axial_drag_coefficient(mach, angles.total_angle_of_attack);
    let cn = normal_force_coefficient(mach, angles.total_angle_of_attack);

    let (_, _, f_total_body) = compute_body_aerodynamic_forces(v_body, q * s, cd, cn);
    let f_total_world = vehicle_orientation.rotate_vector(f_total_body);

    let lever_arm = center_of_pressure - center_of_mass;
    let torque_body = lever_arm.cross(&f_total_body);

    (ForceVector::from_raw(f_total_world), TorqueVector::from_raw(torque_body))
}

pub fn evaluate_vehicle_aerodynamics(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    center_of_mass: Vector3,
    vehicle_orientation: Quaternion,
    relative_airspeed_world: VelocityVector,
    local_speed_of_sound: Speed,
    atmospheric_density: Density
) -> VehicleAerodynamics {
    let s_ref = vehicle_reference_cross_section_area(entries, active_stages);
    let v_rel = relative_airspeed_world.raw();
    let v_mag = relative_airspeed_world.magnitude();
    let mach = mach_number(v_mag, local_speed_of_sound);
    let q = dynamic_pressure(atmospheric_density, v_mag);
    let cop = center_of_pressure(entries, active_stages, mach);

    let angles = compute_aerodynamic_angles(vehicle_orientation, v_rel);
    let cd = axial_drag_coefficient(mach, angles.total_angle_of_attack);
    let cn = normal_force_coefficient(mach, angles.total_angle_of_attack);
    let regime = mach_regime(mach);

    let q_val = q.value();
    let v_body = relative_velocity_body_frame(vehicle_orientation, v_rel);
    let v_body_mag = v_body.magnitude();

    if
        q_val <= 0.0 ||
        s_ref <= 0.0 ||
        v_body_mag < 1e-6 ||
        !q_val.is_finite() ||
        !s_ref.is_finite()
    {
        let lever_arm = cop - center_of_mass;
        return VehicleAerodynamics {
            angles,
            mach,
            mach_regime: regime,
            dynamic_pressure: q,
            drag_coefficient: cd,
            normal_force_coefficient: cn,
            axial_force_body: ForceVector::zero(),
            normal_force_body: ForceVector::zero(),
            total_force_body: ForceVector::zero(),
            total_force_world: ForceVector::zero(),
            torque_body: TorqueVector::zero(),
            torque_world: TorqueVector::zero(),
            center_of_pressure: cop,
            center_of_mass,
            lever_arm,
        };
    }

    let (f_axial_body, f_normal_body, f_total_body) = compute_body_aerodynamic_forces(
        v_body,
        q_val * s_ref,
        cd,
        cn
    );
    let f_total_world = vehicle_orientation.rotate_vector(f_total_body);

    let lever_arm = cop - center_of_mass;
    let tau_body = lever_arm.cross(&f_total_body);
    let tau_world = vehicle_orientation.rotate_vector(tau_body);

    VehicleAerodynamics {
        angles,
        mach,
        mach_regime: regime,
        dynamic_pressure: q,
        drag_coefficient: cd,
        normal_force_coefficient: cn,
        axial_force_body: ForceVector::from_raw(f_axial_body),
        normal_force_body: ForceVector::from_raw(f_normal_body),
        total_force_body: ForceVector::from_raw(f_total_body),
        total_force_world: ForceVector::from_raw(f_total_world),
        torque_body: TorqueVector::from_raw(tau_body),
        torque_world: TorqueVector::from_raw(tau_world),
        center_of_pressure: cop,
        center_of_mass,
        lever_arm,
    }
}
