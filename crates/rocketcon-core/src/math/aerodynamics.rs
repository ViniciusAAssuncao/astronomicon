use crate::domain::{ComponentRecord, VehicleComponentEntry};
use astronomicon_core::units::{
    AngularVelocityVector, Density, ForceVector, Pressure, Speed, Vector3, VelocityVector,
};
use std::f64::consts::PI;

pub fn vehicle_reference_cross_section_area(
    entries_and_records: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
) -> f64 {
    let mut max_diameter = 0.0f64;
    for (entry, record) in entries_and_records {
        if active_stages.contains(&entry.stage_index()) {
            let d = record.component().diameter().value();
            if d.is_finite() && d > max_diameter {
                max_diameter = d;
            }
        }
    }

    if max_diameter <= 0.0 {
        return 0.0;
    }

    let radius = max_diameter * 0.5;
    PI * radius * radius
}

pub fn mach_number(relative_speed: Speed, local_speed_of_sound: Speed) -> f64 {
    let v = relative_speed.value();
    let a = local_speed_of_sound.value();

    if v <= 0.0 || a <= 0.0 || !v.is_finite() || !a.is_finite() {
        0.0
    } else {
        v / a
    }
}

pub fn generic_slender_body_drag_coefficient_estimate(mach: f64) -> f64 {
    if !mach.is_finite() || mach <= 0.0 {
        return 0.20;
    }

    if mach < 0.8 {
        let ratio = mach / 0.8;
        0.20 + 0.05 * ratio * ratio
    } else if mach <= 1.05 {
        let t = (mach - 0.8) / 0.25;
        let smooth = t * t * (3.0 - 2.0 * t);
        0.25 + 0.30 * smooth
    } else {
        let excess = mach - 1.05;
        0.15 + 0.40 / (1.0 + 0.8 * excess.powf(1.15))
    }
}

pub fn drag_coefficient_estimate(mach: f64) -> f64 {
    generic_slender_body_drag_coefficient_estimate(mach)
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
    relative_velocity_direction_world: Vector3,
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

    let v_wind_inertial = topocentric_east * wind_velocity_topocentric.0
        + topocentric_north * wind_velocity_topocentric.1
        + topocentric_up * wind_velocity_topocentric.2;

    let v_atm_inertial = v_corot + v_wind_inertial;
    let v_rel = vehicle_velocity_inertial.raw() - v_atm_inertial;

    VelocityVector::from_raw(v_rel)
}
