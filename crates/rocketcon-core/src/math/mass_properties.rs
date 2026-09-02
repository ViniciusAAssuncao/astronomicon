use crate::domain::{ComponentDetails, ComponentRecord, VehicleComponentEntry};
use crate::math::rigid_body_shapes::{
    solid_cylinder_inertia_tensor, thin_shell_cylinder_inertia_tensor,
};
use astronomicon_core::units::{InertiaTensor, Length, Mass, Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MassProperties {
    pub total_mass: Mass,
    pub center_of_mass: Vector3,
    pub inertia_tensor: InertiaTensor,
}

impl MassProperties {
    pub fn new(
        total_mass: Mass,
        center_of_mass: Vector3,
        inertia_tensor: InertiaTensor,
    ) -> Self {
        Self {
            total_mass,
            center_of_mass,
            inertia_tensor,
        }
    }

    pub fn total_mass(&self) -> Mass {
        self.total_mass
    }

    pub fn center_of_mass(&self) -> Vector3 {
        self.center_of_mass
    }

    pub fn inertia_tensor(&self) -> InertiaTensor {
        self.inertia_tensor
    }
}

pub fn resolve_mass_properties(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    propellant_load_fraction: f64,
    payload_masses: &HashMap<Uuid, Mass>,
) -> MassProperties {
    let load_frac = propellant_load_fraction.clamp(0.0, 1.0);
    let mut total_mass_val = 0.0;
    let mut sum_mr = Vector3::zero();

    let mut component_masses = Vec::with_capacity(entries.len());

    for (entry, record) in entries {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        let comp = record.component();
        let mut m = comp.dry_mass().value();

        if let ComponentDetails::PropellantTank(tank) = record.details() {
            m += tank.max_propellant_mass().value() * load_frac;
        }

        if let Some(payload_m) = payload_masses
            .get(&entry.id())
            .or_else(|| payload_masses.get(&entry.component_id()))
        {
            let p_val = payload_m.value();
            if p_val.is_finite() && p_val > 0.0 {
                m += p_val;
            }
        }

        if m.is_finite() && m > 0.0 {
            total_mass_val += m;
            sum_mr = sum_mr + entry.mount_offset() * m;
            component_masses.push((entry, record, m));
        }
    }

    if total_mass_val <= 0.0 || !total_mass_val.is_finite() {
        return MassProperties {
            total_mass: Mass::new(0.0),
            center_of_mass: Vector3::zero(),
            inertia_tensor: InertiaTensor::zero(),
        };
    }

    let center_of_mass = sum_mr / total_mass_val;
    let mut total_inertia = InertiaTensor::zero();

    for (entry, record, m) in component_masses {
        let comp = record.component();
        let radius = Length::new(comp.diameter().value() * 0.5);
        let length = comp.length();
        let mass = Mass::new(m);

        let i_local = match record.details() {
            ComponentDetails::PropellantTank(_) => {
                thin_shell_cylinder_inertia_tensor(mass, radius, length)
            }
            _ => solid_cylinder_inertia_tensor(mass, radius, length),
        };

        let i_rotated = match entry.actuation_axis() {
            Some(axis) => {
                let rot_q = Quaternion::from_rotation_between(Vector3::new(0.0, 0.0, 1.0), axis);
                let rot_m = rot_q.to_rotation_matrix();
                i_local.rotate_by(&rot_m)
            }
            None => i_local,
        };

        let d = entry.mount_offset() - center_of_mass;
        let i_shifted = i_rotated.parallel_axis_shift(mass, d);
        total_inertia = total_inertia.add(&i_shifted);
    }

    MassProperties {
        total_mass: Mass::new(total_mass_val),
        center_of_mass,
        inertia_tensor: total_inertia,
    }
}

pub fn resolve_mass_properties_without_payloads(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    propellant_load_fraction: f64,
) -> MassProperties {
    let empty = HashMap::new();
    resolve_mass_properties(entries, active_stages, propellant_load_fraction, &empty)
}

pub fn mass_properties(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    propellant_load_fraction: f64,
    payload_masses: &HashMap<Uuid, Mass>,
) -> MassProperties {
    resolve_mass_properties(entries, active_stages, propellant_load_fraction, payload_masses)
}
