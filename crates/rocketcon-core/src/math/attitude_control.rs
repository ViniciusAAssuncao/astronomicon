use crate::domain::{ComponentDetails, ComponentRecord, VehicleComponentEntry};
use astronomicon_core::units::{AngularMomentum, Torque, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AttitudeControlAuthority {
    pub gimbal_torque_x: Torque,
    pub gimbal_torque_y: Torque,
    pub gimbal_torque_z: Torque,
    pub rcs_torque_x: Torque,
    pub rcs_torque_y: Torque,
    pub rcs_torque_z: Torque,
    pub reaction_wheel_torque_x: Torque,
    pub reaction_wheel_torque_y: Torque,
    pub reaction_wheel_torque_z: Torque,
    pub reaction_wheel_momentum_x: AngularMomentum,
    pub reaction_wheel_momentum_y: AngularMomentum,
    pub reaction_wheel_momentum_z: AngularMomentum,
    pub is_controllable_x: bool,
    pub is_controllable_y: bool,
    pub is_controllable_z: bool,
}

impl AttitudeControlAuthority {
    pub fn new(
        gimbal_torque_x: Torque,
        gimbal_torque_y: Torque,
        gimbal_torque_z: Torque,
        rcs_torque_x: Torque,
        rcs_torque_y: Torque,
        rcs_torque_z: Torque,
        reaction_wheel_torque_x: Torque,
        reaction_wheel_torque_y: Torque,
        reaction_wheel_torque_z: Torque,
        reaction_wheel_momentum_x: AngularMomentum,
        reaction_wheel_momentum_y: AngularMomentum,
        reaction_wheel_momentum_z: AngularMomentum,
        is_controllable_x: bool,
        is_controllable_y: bool,
        is_controllable_z: bool,
    ) -> Self {
        Self {
            gimbal_torque_x,
            gimbal_torque_y,
            gimbal_torque_z,
            rcs_torque_x,
            rcs_torque_y,
            rcs_torque_z,
            reaction_wheel_torque_x,
            reaction_wheel_torque_y,
            reaction_wheel_torque_z,
            reaction_wheel_momentum_x,
            reaction_wheel_momentum_y,
            reaction_wheel_momentum_z,
            is_controllable_x,
            is_controllable_y,
            is_controllable_z,
        }
    }
}

pub fn resolve_attitude_control_authority(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    center_of_mass: Vector3,
) -> AttitudeControlAuthority {
    let mut gimbal_tx = 0.0;
    let mut gimbal_ty = 0.0;
    let mut gimbal_tz = 0.0;

    let mut rcs_tx = 0.0;
    let mut rcs_ty = 0.0;
    let mut rcs_tz = 0.0;

    let mut rw_tx = 0.0;
    let mut rw_ty = 0.0;
    let mut rw_tz = 0.0;

    let mut rw_hx = 0.0;
    let mut rw_hy = 0.0;
    let mut rw_hz = 0.0;

    let body_axes = [
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];

    for (entry, record) in entries {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        let r = entry.mount_offset() - center_of_mass;

        match record.details() {
            ComponentDetails::Engine(engine) => {
                if engine.has_gimbal() {
                    if let (Some(deflection), Some(act_axis)) =
                        (engine.max_gimbal_deflection(), entry.actuation_axis())
                    {
                        let thrust = engine.max_thrust().value();
                        let delta = deflection.value();

                        let mut max_tx = 0.0f64;
                        let mut max_ty = 0.0f64;
                        let mut max_tz = 0.0f64;

                        for rot_axis in &body_axes {
                            let dir_pos = act_axis.rotate_about_axis(*rot_axis, delta);
                            let dir_neg = act_axis.rotate_about_axis(*rot_axis, -delta);

                            let force_pos = dir_pos * thrust;
                            let force_neg = dir_neg * thrust;

                            let tau_pos = r.cross(&force_pos);
                            let tau_neg = r.cross(&force_neg);

                            max_tx = max_tx.max(tau_pos.0.abs()).max(tau_neg.0.abs());
                            max_ty = max_ty.max(tau_pos.1.abs()).max(tau_neg.1.abs());
                            max_tz = max_tz.max(tau_pos.2.abs()).max(tau_neg.2.abs());
                        }

                        gimbal_tx += max_tx;
                        gimbal_ty += max_ty;
                        gimbal_tz += max_tz;
                    }
                }
            }
            ComponentDetails::ReactionControlThruster(rcs) => {
                if let Some(act_axis) = entry.actuation_axis() {
                    let thrust = rcs.max_thrust().value();
                    let force = act_axis * thrust;
                    let tau = r.cross(&force);

                    rcs_tx += tau.0.abs();
                    rcs_ty += tau.1.abs();
                    rcs_tz += tau.2.abs();
                }
            }
            ComponentDetails::ReactionWheel(rw) => {
                if let Some(act_axis) = entry.actuation_axis() {
                    let max_torque = rw.max_torque().value();
                    let max_momentum = rw.max_angular_momentum_storage().value();

                    rw_tx += act_axis.0.abs() * max_torque;
                    rw_ty += act_axis.1.abs() * max_torque;
                    rw_tz += act_axis.2.abs() * max_torque;

                    rw_hx += act_axis.0.abs() * max_momentum;
                    rw_hy += act_axis.1.abs() * max_momentum;
                    rw_hz += act_axis.2.abs() * max_momentum;
                }
            }
            _ => {}
        }
    }

    let is_controllable_x = gimbal_tx > 0.0 || rcs_tx > 0.0 || rw_tx > 0.0;
    let is_controllable_y = gimbal_ty > 0.0 || rcs_ty > 0.0 || rw_ty > 0.0;
    let is_controllable_z = gimbal_tz > 0.0 || rcs_tz > 0.0 || rw_tz > 0.0;

    AttitudeControlAuthority {
        gimbal_torque_x: Torque::new(gimbal_tx),
        gimbal_torque_y: Torque::new(gimbal_ty),
        gimbal_torque_z: Torque::new(gimbal_tz),
        rcs_torque_x: Torque::new(rcs_tx),
        rcs_torque_y: Torque::new(rcs_ty),
        rcs_torque_z: Torque::new(rcs_tz),
        reaction_wheel_torque_x: Torque::new(rw_tx),
        reaction_wheel_torque_y: Torque::new(rw_ty),
        reaction_wheel_torque_z: Torque::new(rw_tz),
        reaction_wheel_momentum_x: AngularMomentum::new(rw_hx),
        reaction_wheel_momentum_y: AngularMomentum::new(rw_hy),
        reaction_wheel_momentum_z: AngularMomentum::new(rw_hz),
        is_controllable_x,
        is_controllable_y,
        is_controllable_z,
    }
}

pub fn attitude_control_authority(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    center_of_mass: Vector3,
) -> AttitudeControlAuthority {
    resolve_attitude_control_authority(entries, active_stages, center_of_mass)
}