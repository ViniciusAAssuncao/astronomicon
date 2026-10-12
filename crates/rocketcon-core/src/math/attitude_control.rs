use crate::domain::{
    ComponentDetails,
    ComponentRecord,
    ReactionControlThrusterSpecification,
    VehicleComponentEntry,
    VehicleControlInput,
};
use astronomicon_core::units::{ AngularMomentum, Force, Torque, Vector3 };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use uuid::Uuid;

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
        is_controllable_z: bool
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RcsThrusterMapping {
    pub vehicle_component_id: Uuid,
    pub component_id: Uuid,
    pub stage_index: u32,
    pub max_thrust: Force,
    pub thrust_axis_body: Vector3,
    pub lever_arm_body: Vector3,
    pub torque_per_unit_thrust: Vector3,
    pub max_torque_body: Vector3,
    pub max_force_body: Vector3,
}

impl RcsThrusterMapping {
    pub fn new(
        vehicle_component_id: Uuid,
        component_id: Uuid,
        stage_index: u32,
        spec: &ReactionControlThrusterSpecification,
        mount_offset: Vector3,
        actuation_axis: Option<Vector3>,
        center_of_mass: Vector3
    ) -> Self {
        let thrust_axis = actuation_axis.unwrap_or(Vector3::new(0.0, 0.0, 1.0)).normalized();
        let lever_arm = mount_offset - center_of_mass;
        let max_thrust = spec.max_thrust();
        let max_force = thrust_axis * max_thrust.value();
        let torque_per_unit_thrust = lever_arm.cross(&thrust_axis);
        let max_torque = lever_arm.cross(&max_force);

        Self {
            vehicle_component_id,
            component_id,
            stage_index,
            max_thrust,
            thrust_axis_body: thrust_axis,
            lever_arm_body: lever_arm,
            torque_per_unit_thrust,
            max_torque_body: max_torque,
            max_force_body: max_force,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RcsControlAllocationMatrix {
    pub thrusters: Vec<RcsThrusterMapping>,
    pub max_positive_torque: Vector3,
    pub max_negative_torque: Vector3,
    pub max_positive_force: Vector3,
    pub max_negative_force: Vector3,
}

impl RcsControlAllocationMatrix {
    pub fn from_components(
        entries: &[(VehicleComponentEntry, ComponentRecord)],
        active_stages: &[u32],
        center_of_mass: Vector3
    ) -> Self {
        let mut thrusters = Vec::new();
        let mut max_pos_torque = Vector3::zero();
        let mut max_neg_torque = Vector3::zero();
        let mut max_pos_force = Vector3::zero();
        let mut max_neg_force = Vector3::zero();

        for (entry, record) in entries {
            if !active_stages.contains(&entry.stage_index()) {
                continue;
            }

            if let ComponentDetails::ReactionControlThruster(spec) = record.details() {
                let mapping = RcsThrusterMapping::new(
                    entry.id(),
                    entry.component_id(),
                    entry.stage_index(),
                    spec,
                    entry.mount_offset(),
                    entry.actuation_axis(),
                    center_of_mass
                );

                let tau = mapping.max_torque_body;
                if tau.0 > 0.0 {
                    max_pos_torque.0 += tau.0;
                } else {
                    max_neg_torque.0 += tau.0.abs();
                }
                if tau.1 > 0.0 {
                    max_pos_torque.1 += tau.1;
                } else {
                    max_neg_torque.1 += tau.1.abs();
                }
                if tau.2 > 0.0 {
                    max_pos_torque.2 += tau.2;
                } else {
                    max_neg_torque.2 += tau.2.abs();
                }

                let f = mapping.max_force_body;
                if f.0 > 0.0 {
                    max_pos_force.0 += f.0;
                } else {
                    max_neg_force.0 += f.0.abs();
                }
                if f.1 > 0.0 {
                    max_pos_force.1 += f.1;
                } else {
                    max_neg_force.1 += f.1.abs();
                }
                if f.2 > 0.0 {
                    max_pos_force.2 += f.2;
                } else {
                    max_neg_force.2 += f.2.abs();
                }

                thrusters.push(mapping);
            }
        }

        Self {
            thrusters,
            max_positive_torque: max_pos_torque,
            max_negative_torque: max_neg_torque,
            max_positive_force: max_pos_force,
            max_negative_force: max_neg_force,
        }
    }

    pub fn allocate_throttles(&self, control_input: &VehicleControlInput) -> HashMap<Uuid, f64> {
        let mut throttles = HashMap::with_capacity(self.thrusters.len());

        let target_torque = if let Some(t) = control_input.target_attitude_torque {
            Some(t)
        } else if
            control_input.pitch.is_some() ||
            control_input.yaw.is_some() ||
            control_input.roll.is_some()
        {
            let p = control_input.pitch.unwrap_or(0.0);
            let y = control_input.yaw.unwrap_or(0.0);
            let r = control_input.roll.unwrap_or(0.0);

            let tx = if p >= 0.0 {
                p * self.max_positive_torque.0
            } else {
                p * self.max_negative_torque.0
            };
            let ty = if y >= 0.0 {
                y * self.max_positive_torque.1
            } else {
                y * self.max_negative_torque.1
            };
            let tz = if r >= 0.0 {
                r * self.max_positive_torque.2
            } else {
                r * self.max_negative_torque.2
            };

            Some(Vector3::new(tx, ty, tz))
        } else {
            None
        };

        let target_translation = control_input.target_translation_force;

        for thruster in &self.thrusters {
            if
                let Some(cmd) = control_input
                    .command_for(&thruster.vehicle_component_id)
                    .or_else(|| control_input.command_for(&thruster.component_id))
            {
                if let Some(throttle) = cmd.target_rcs_throttle {
                    throttles.insert(thruster.vehicle_component_id, throttle.clamp(0.0, 1.0));
                    continue;
                }
            }

            let mut u_rot = 0.0f64;
            if let Some(tau_des) = target_torque {
                let tau_t = thruster.max_torque_body;
                let dot = tau_t.dot(&tau_des);

                if dot > 1e-9 {
                    let mut sum_comp = 0.0f64;
                    let mut comp_count = 0.0f64;

                    if tau_des.0.abs() > 1e-6 {
                        comp_count += 1.0;
                        if tau_des.0 > 0.0 && tau_t.0 > 0.0 && self.max_positive_torque.0 > 1e-6 {
                            sum_comp +=
                                (tau_des.0 / self.max_positive_torque.0) *
                                (tau_t.0 / self.max_positive_torque.0.max(tau_t.0));
                        } else if
                            tau_des.0 < 0.0 &&
                            tau_t.0 < 0.0 &&
                            self.max_negative_torque.0 > 1e-6
                        {
                            sum_comp +=
                                (tau_des.0.abs() / self.max_negative_torque.0) *
                                (tau_t.0.abs() / self.max_negative_torque.0.max(tau_t.0.abs()));
                        }
                    }

                    if tau_des.1.abs() > 1e-6 {
                        comp_count += 1.0;
                        if tau_des.1 > 0.0 && tau_t.1 > 0.0 && self.max_positive_torque.1 > 1e-6 {
                            sum_comp +=
                                (tau_des.1 / self.max_positive_torque.1) *
                                (tau_t.1 / self.max_positive_torque.1.max(tau_t.1));
                        } else if
                            tau_des.1 < 0.0 &&
                            tau_t.1 < 0.0 &&
                            self.max_negative_torque.1 > 1e-6
                        {
                            sum_comp +=
                                (tau_des.1.abs() / self.max_negative_torque.1) *
                                (tau_t.1.abs() / self.max_negative_torque.1.max(tau_t.1.abs()));
                        }
                    }

                    if tau_des.2.abs() > 1e-6 {
                        comp_count += 1.0;
                        if tau_des.2 > 0.0 && tau_t.2 > 0.0 && self.max_positive_torque.2 > 1e-6 {
                            sum_comp +=
                                (tau_des.2 / self.max_positive_torque.2) *
                                (tau_t.2 / self.max_positive_torque.2.max(tau_t.2));
                        } else if
                            tau_des.2 < 0.0 &&
                            tau_t.2 < 0.0 &&
                            self.max_negative_torque.2 > 1e-6
                        {
                            sum_comp +=
                                (tau_des.2.abs() / self.max_negative_torque.2) *
                                (tau_t.2.abs() / self.max_negative_torque.2.max(tau_t.2.abs()));
                        }
                    }

                    if comp_count > 0.0 {
                        u_rot = (sum_comp / comp_count.sqrt()).clamp(0.0, 1.0);
                    }
                }
            }

            let mut u_trans = 0.0f64;
            if let Some(f_des) = target_translation {
                let f_t = thruster.max_force_body;
                let dot_f = f_t.dot(&f_des);
                if dot_f > 1e-9 {
                    let mut sum_f = 0.0f64;
                    let mut count_f = 0.0f64;

                    if f_des.0.abs() > 1e-6 {
                        count_f += 1.0;
                        if f_des.0 > 0.0 && f_t.0 > 0.0 && self.max_positive_force.0 > 1e-6 {
                            sum_f += f_des.0 / self.max_positive_force.0;
                        } else if f_des.0 < 0.0 && f_t.0 < 0.0 && self.max_negative_force.0 > 1e-6 {
                            sum_f += f_des.0.abs() / self.max_negative_force.0;
                        }
                    }

                    if f_des.1.abs() > 1e-6 {
                        count_f += 1.0;
                        if f_des.1 > 0.0 && f_t.1 > 0.0 && self.max_positive_force.1 > 1e-6 {
                            sum_f += f_des.1 / self.max_positive_force.1;
                        } else if f_des.1 < 0.0 && f_t.1 < 0.0 && self.max_negative_force.1 > 1e-6 {
                            sum_f += f_des.1.abs() / self.max_negative_force.1;
                        }
                    }

                    if f_des.2.abs() > 1e-6 {
                        count_f += 1.0;
                        if f_des.2 > 0.0 && f_t.2 > 0.0 && self.max_positive_force.2 > 1e-6 {
                            sum_f += f_des.2 / self.max_positive_force.2;
                        } else if f_des.2 < 0.0 && f_t.2 < 0.0 && self.max_negative_force.2 > 1e-6 {
                            sum_f += f_des.2.abs() / self.max_negative_force.2;
                        }
                    }

                    if count_f > 0.0 {
                        u_trans = (sum_f / count_f).clamp(0.0, 1.0);
                    }
                }
            }

            let total_u = (u_rot + u_trans).clamp(0.0, 1.0);
            throttles.insert(thruster.vehicle_component_id, total_u);
        }

        throttles
    }
}

pub fn build_rcs_allocation_matrix(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    center_of_mass: Vector3
) -> RcsControlAllocationMatrix {
    RcsControlAllocationMatrix::from_components(entries, active_stages, center_of_mass)
}

pub fn resolve_attitude_control_authority(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    center_of_mass: Vector3
) -> AttitudeControlAuthority {
    let mut gimbal_tx = 0.0;
    let mut gimbal_ty = 0.0;
    let mut gimbal_tz = 0.0;

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
                    if let Some(deflection) = engine.max_gimbal_deflection() {
                        let act_axis = entry
                            .actuation_axis()
                            .unwrap_or(Vector3::new(0.0, 0.0, 1.0))
                            .normalized();
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
            ComponentDetails::ReactionWheel(rw) => {
                let act_axis = entry
                    .actuation_axis()
                    .unwrap_or(Vector3::new(0.0, 0.0, 1.0))
                    .normalized();
                let max_torque = rw.max_torque().value();
                let max_momentum = rw.max_angular_momentum_storage().value();

                rw_tx += act_axis.0.abs() * max_torque;
                rw_ty += act_axis.1.abs() * max_torque;
                rw_tz += act_axis.2.abs() * max_torque;

                rw_hx += act_axis.0.abs() * max_momentum;
                rw_hy += act_axis.1.abs() * max_momentum;
                rw_hz += act_axis.2.abs() * max_momentum;
            }
            _ => {}
        }
    }

    let rcs_matrix = RcsControlAllocationMatrix::from_components(
        entries,
        active_stages,
        center_of_mass
    );
    let rcs_tx = rcs_matrix.max_positive_torque.0.max(rcs_matrix.max_negative_torque.0);
    let rcs_ty = rcs_matrix.max_positive_torque.1.max(rcs_matrix.max_negative_torque.1);
    let rcs_tz = rcs_matrix.max_positive_torque.2.max(rcs_matrix.max_negative_torque.2);

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
    center_of_mass: Vector3
) -> AttitudeControlAuthority {
    resolve_attitude_control_authority(entries, active_stages, center_of_mass)
}
