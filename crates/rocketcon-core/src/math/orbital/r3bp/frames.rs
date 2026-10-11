use super::types::{Cr3bpParameters, SynodicState};
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};

pub fn inertial_to_synodic_state(
    inertial_position: Position,
    inertial_velocity: VelocityVector,
    epoch: Duration,
    params: &Cr3bpParameters,
) -> SynodicState {
    let l_star = params.characteristic_length.value();
    let t_star = params.characteristic_time.value();
    if l_star <= 0.0 || t_star <= 0.0 {
        return SynodicState::zero();
    }

    let t_dim = epoch.value();
    let theta = t_dim / t_star;

    let r_inertial_norm = inertial_position.raw() / l_star;
    let v_inertial_norm = inertial_velocity.raw() / (l_star / t_star);

    let r_rot = r_inertial_norm.rotate_about_z(-theta);
    let v_rot = v_inertial_norm.rotate_about_z(-theta);

    let omega_cross_r = Vector3::new(-r_rot.1, r_rot.0, 0.0);
    let v_syn = v_rot - omega_cross_r;

    SynodicState::new(r_rot, v_syn)
}

pub fn synodic_to_inertial_state(
    synodic_state: &SynodicState,
    epoch: Duration,
    params: &Cr3bpParameters,
) -> (Position, VelocityVector) {
    let l_star = params.characteristic_length.value();
    let t_star = params.characteristic_time.value();
    if l_star <= 0.0 || t_star <= 0.0 {
        return (Position::zero(), VelocityVector::zero());
    }

    let t_dim = epoch.value();
    let theta = t_dim / t_star;

    let r_syn = synodic_state.position;
    let v_syn = synodic_state.velocity;

    let omega_cross_r = Vector3::new(-r_syn.1, r_syn.0, 0.0);
    let v_rot = v_syn + omega_cross_r;

    let r_inertial_norm = r_syn.rotate_about_z(theta);
    let v_inertial_norm = v_rot.rotate_about_z(theta);

    let r_inertial = r_inertial_norm * l_star;
    let v_inertial = v_inertial_norm * (l_star / t_star);

    (
        Position::from_raw(r_inertial),
        VelocityVector::from_raw(v_inertial),
    )
}