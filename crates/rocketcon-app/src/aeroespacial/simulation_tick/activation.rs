use rocketcon_core::domain::{
    ComponentDetails, ComponentRecord, VehicleComponentEntry, VehicleControlInput, VehicleSnapshot,
};

pub(super) fn is_propulsion_or_control_active(
    snapshot: &VehicleSnapshot,
    components: &[(VehicleComponentEntry, ComponentRecord)],
    control_input: &VehicleControlInput,
) -> bool {
    if let Some(att) = control_input.attitude_demand_vector() {
        if att.magnitude() > 1e-4 {
            return true;
        }
    }
    if let Some(trans) = control_input.target_translation_force {
        if trans.magnitude() > 1e-4 {
            return true;
        }
    }

    for (entry, record) in components {
        if !snapshot.is_stage_active(entry.stage_index()) {
            continue;
        }

        if let Some(cmd) = control_input
            .command_for(&entry.id())
            .or_else(|| control_input.command_for(&entry.component_id()))
        {
            if cmd
                .target_reaction_wheel_torque_fraction
                .map_or(false, |f| f.abs() > 1e-4)
            {
                return true;
            }
            if cmd.target_gimbal_pitch.is_some() || cmd.target_gimbal_yaw.is_some() {
                return true;
            }
            if cmd.target_rcs_throttle.map_or(false, |f| f.abs() > 1e-4) {
                return true;
            }
        }

        match record.details() {
            ComponentDetails::Engine(_) => {
                if let Some(op) = snapshot.engine_operational_states().get(&entry.id()) {
                    if op.load_fraction() > 1e-4 {
                        return true;
                    }
                }
            }
            ComponentDetails::ReactionControlThruster(_) => {
                if let Some(op) = snapshot.engine_operational_states().get(&entry.id()) {
                    if op.load_fraction() > 1e-4 {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}
