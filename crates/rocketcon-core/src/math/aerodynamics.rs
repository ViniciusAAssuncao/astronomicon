use crate::domain::{ ComponentKind, ComponentRecord, VehicleComponentEntry };
use astronomicon_core::units::{
    Angle,
    AngularVelocityVector,
    Density,
    ForceVector,
    Pressure,
    Quaternion,
    Speed,
    TorqueVector,
    Vector3,
    VelocityVector,
};
use serde::{ Deserialize, Serialize };
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MachRegime {
    Subsonic,
    Transonic,
    Supersonic,
    Hypersonic,
}

pub fn mach_regime(mach: f64) -> MachRegime {
    if !mach.is_finite() || mach < 0.8 {
        MachRegime::Subsonic
    } else if mach <= 1.2 {
        MachRegime::Transonic
    } else if mach < 5.0 {
        MachRegime::Supersonic
    } else {
        MachRegime::Hypersonic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AerodynamicAngles {
    pub angle_of_attack: Angle,
    pub sideslip_angle: Angle,
    pub total_angle_of_attack: Angle,
}

impl AerodynamicAngles {
    pub fn new(
        angle_of_attack: Angle,
        sideslip_angle: Angle,
        total_angle_of_attack: Angle
    ) -> Self {
        Self {
            angle_of_attack,
            sideslip_angle,
            total_angle_of_attack,
        }
    }

    pub fn zero() -> Self {
        Self {
            angle_of_attack: Angle::new(0.0),
            sideslip_angle: Angle::new(0.0),
            total_angle_of_attack: Angle::new(0.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleAerodynamics {
    pub angles: AerodynamicAngles,
    pub mach: f64,
    pub mach_regime: MachRegime,
    pub dynamic_pressure: Pressure,
    pub drag_coefficient: f64,
    pub normal_force_coefficient: f64,
    pub axial_force_body: ForceVector,
    pub normal_force_body: ForceVector,
    pub total_force_body: ForceVector,
    pub total_force_world: ForceVector,
    pub torque_body: TorqueVector,
    pub torque_world: TorqueVector,
    pub center_of_pressure: Vector3,
    pub center_of_mass: Vector3,
    pub lever_arm: Vector3,
}

impl VehicleAerodynamics {
    pub fn angles(&self) -> AerodynamicAngles {
        self.angles
    }

    pub fn mach(&self) -> f64 {
        self.mach
    }

    pub fn mach_regime(&self) -> MachRegime {
        self.mach_regime
    }

    pub fn dynamic_pressure(&self) -> Pressure {
        self.dynamic_pressure
    }

    pub fn drag_coefficient(&self) -> f64 {
        self.drag_coefficient
    }

    pub fn normal_force_coefficient(&self) -> f64 {
        self.normal_force_coefficient
    }

    pub fn axial_force_body(&self) -> ForceVector {
        self.axial_force_body
    }

    pub fn normal_force_body(&self) -> ForceVector {
        self.normal_force_body
    }

    pub fn total_force_body(&self) -> ForceVector {
        self.total_force_body
    }

    pub fn total_force_world(&self) -> ForceVector {
        self.total_force_world
    }

    pub fn torque_body(&self) -> TorqueVector {
        self.torque_body
    }

    pub fn torque_world(&self) -> TorqueVector {
        self.torque_world
    }

    pub fn center_of_pressure(&self) -> Vector3 {
        self.center_of_pressure
    }

    pub fn center_of_mass(&self) -> Vector3 {
        self.center_of_mass
    }

    pub fn lever_arm(&self) -> Vector3 {
        self.lever_arm
    }
}

pub fn vehicle_reference_cross_section_area(
    entries_and_records: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32]
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

pub fn relative_velocity_body_frame(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3
) -> Vector3 {
    vehicle_orientation.inverse().rotate_vector(relative_airspeed_world)
}

pub fn decompose_relative_velocity(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3
) -> (Vector3, Vector3) {
    let v_body = relative_velocity_body_frame(vehicle_orientation, relative_airspeed_world);
    let axial = Vector3::new(0.0, 0.0, v_body.2);
    let lateral = Vector3::new(v_body.0, v_body.1, 0.0);
    (axial, lateral)
}

pub fn decompose_relative_velocity_world(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3
) -> (Vector3, Vector3) {
    let (axial_body, lateral_body) = decompose_relative_velocity(
        vehicle_orientation,
        relative_airspeed_world
    );
    (vehicle_orientation.rotate_vector(axial_body), vehicle_orientation.rotate_vector(lateral_body))
}

pub fn compute_aerodynamic_angles(
    vehicle_orientation: Quaternion,
    relative_airspeed_world: Vector3
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
    relative_airspeed_world: Vector3
) -> AerodynamicAngles {
    compute_aerodynamic_angles(vehicle_orientation, relative_airspeed_world)
}

pub fn resolve_center_of_pressure(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    mach: f64
) -> Vector3 {
    let mut total_weight = 0.0f64;
    let mut weighted_cop = Vector3::zero();
    let mut min_z = f64::INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    let mut count = 0usize;

    for (entry, record) in entries {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        count += 1;
        let comp = record.component();
        let offset = entry.mount_offset();
        let length = comp.length().value().max(0.01);
        let diameter = comp.diameter().value().max(0.01);
        let radius = diameter * 0.5;
        let base_area = PI * radius * radius;
        let planform_area = diameter * length;

        let z_center = offset.2;
        let z_min = z_center - length * 0.5;
        let z_max = z_center + length * 0.5;

        if z_min < min_z {
            min_z = z_min;
        }
        if z_max > max_z {
            max_z = z_max;
        }

        let (weight, local_cop_z) = match comp.kind() {
            ComponentKind::PayloadFairing => {
                let w = 2.0 * base_area;
                let z_cop = z_center + length * 0.16666666666666666;
                (w, z_cop)
            }
            ComponentKind::Engine => {
                let w = 1.5 * base_area + 0.5 * planform_area;
                let z_cop = z_center - length * 0.25;
                (w, z_cop)
            }
            ComponentKind::PropellantTank => {
                let w = 1.1 * planform_area;
                (w, z_center)
            }
            ComponentKind::SolarPanel | ComponentKind::Radiator => {
                let w = 1.0 * planform_area;
                (w, z_center)
            }
            _ => {
                let w = 0.5 * planform_area + 0.5 * base_area;
                (w, z_center)
            }
        };

        let comp_cop = Vector3::new(offset.0, offset.1, local_cop_z);
        weighted_cop = weighted_cop + comp_cop * weight;
        total_weight += weight;
    }

    if count == 0 || total_weight <= 0.0 || !total_weight.is_finite() {
        return Vector3::zero();
    }

    let mut cop = weighted_cop / total_weight;

    let total_length = (max_z - min_z).max(0.0);
    if total_length > 0.0 && mach > 0.8 && mach.is_finite() {
        let factor = if mach < 2.0 { ((mach - 0.8) / 1.2) * 0.12 } else { 0.12 };
        let aft_shift = total_length * factor;
        cop.2 -= aft_shift;
        if cop.2 < min_z {
            cop.2 = min_z;
        }
    }

    cop
}

pub fn center_of_pressure(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    mach: f64
) -> Vector3 {
    resolve_center_of_pressure(entries, active_stages, mach)
}

pub fn zero_lift_drag_coefficient(mach: f64) -> f64 {
    if !mach.is_finite() || mach <= 0.0 {
        return 0.18;
    }

    match mach_regime(mach) {
        MachRegime::Subsonic => {
            let m_norm = mach / 0.8;
            0.18 + 0.04 * m_norm * m_norm
        }
        MachRegime::Transonic => {
            let t = (mach - 0.8) / 0.4;
            let wave_peak = (PI * t).sin().powi(2);
            0.22 + 0.38 * wave_peak + 0.15 * t
        }
        MachRegime::Supersonic => {
            let beta = (mach * mach - 1.0_f64).max(0.1_f64).sqrt();
            0.18 + 0.35 / beta
        }
        MachRegime::Hypersonic => {
            let beta_ref = 4.898979485566356_f64;
            let base_hypersonic = 0.18 + 0.35 / beta_ref;
            (base_hypersonic - 0.02 * ((mach - 5.0) / 10.0)).max(0.18)
        }
    }
}

pub fn normal_force_slope(mach: f64) -> f64 {
    if !mach.is_finite() || mach <= 0.0 {
        return 2.0;
    }

    match mach_regime(mach) {
        MachRegime::Subsonic => 2.0,
        MachRegime::Transonic => {
            let t = (mach - 0.8) / 0.4;
            2.0 + 0.6 * (PI * t).sin()
        }
        MachRegime::Supersonic => {
            let beta = (mach * mach - 1.0_f64).max(0.2_f64).sqrt();
            2.0 / beta
        }
        MachRegime::Hypersonic => {
            let beta_hyp = 4.898979485566356_f64;
            (2.0 / beta_hyp).max(0.4)
        }
    }
}

pub fn normal_force_coefficient(mach: f64, total_angle_of_attack: Angle) -> f64 {
    let alpha = total_angle_of_attack.value();
    if !alpha.is_finite() {
        return 0.0;
    }

    let sin_a = alpha.sin();
    let cos_a = alpha.cos();
    let abs_sin_a = sin_a.abs();

    match mach_regime(mach) {
        MachRegime::Hypersonic => {
            2.0 * abs_sin_a * abs_sin_a * cos_a.max(0.0) + 1.2 * abs_sin_a * sin_a
        }
        _ => {
            let cn_alpha = normal_force_slope(mach);
            let linear_term = cn_alpha * sin_a * cos_a;
            let crossflow_term = 1.2 * abs_sin_a * sin_a;
            linear_term + crossflow_term
        }
    }
}

pub fn axial_drag_coefficient(mach: f64, total_angle_of_attack: Angle) -> f64 {
    let cd0 = zero_lift_drag_coefficient(mach);
    let alpha = total_angle_of_attack.value();
    if !alpha.is_finite() {
        return cd0;
    }

    let sin_a = alpha.sin();
    let cos_a = alpha.cos();
    let abs_sin_a = sin_a.abs();
    let cn = normal_force_coefficient(mach, total_angle_of_attack);

    let induced_drag = (cn * sin_a).abs();
    let crossflow_drag = 1.2 * abs_sin_a * abs_sin_a * abs_sin_a;

    (cd0 * cos_a.abs() + induced_drag + crossflow_drag).max(0.01)
}

pub fn generic_slender_body_drag_coefficient_estimate(mach: f64) -> f64 {
    axial_drag_coefficient(mach, Angle::new(0.0))
}

pub fn drag_coefficient_estimate(mach: f64) -> f64 {
    axial_drag_coefficient(mach, Angle::new(0.0))
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

    let f_axial_mag = q * s * cd;
    let f_normal_mag = q * s * cn.abs();

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

    let f_axial_mag = q_val * s_ref * cd;
    let f_normal_mag = q_val * s_ref * cn.abs();

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

pub fn local_atmospheric_relative_velocity(
    vehicle_velocity_inertial: VelocityVector,
    planet_angular_velocity_inertial: AngularVelocityVector,
    vehicle_position_relative_to_planet_center_inertial: Vector3,
    wind_velocity_topocentric: Vector3,
    topocentric_east: Vector3,
    topocentric_north: Vector3,
    topocentric_up: Vector3
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
