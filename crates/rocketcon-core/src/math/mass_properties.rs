use crate::domain::{ComponentDetails, ComponentRecord, VehicleComponentEntry};
use astronomicon_core::units::{Mass, MomentOfInertia, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MassProperties {
    pub total_mass: Mass,
    pub center_of_mass: Vector3,
    pub moment_of_inertia_x: MomentOfInertia,
    pub moment_of_inertia_y: MomentOfInertia,
    pub moment_of_inertia_z: MomentOfInertia,
}

impl MassProperties {
    pub fn new(
        total_mass: Mass,
        center_of_mass: Vector3,
        moment_of_inertia_x: MomentOfInertia,
        moment_of_inertia_y: MomentOfInertia,
        moment_of_inertia_z: MomentOfInertia,
    ) -> Self {
        Self {
            total_mass,
            center_of_mass,
            moment_of_inertia_x,
            moment_of_inertia_y,
            moment_of_inertia_z,
        }
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
            component_masses.push((m, entry.mount_offset()));
        }
    }

    if total_mass_val <= 0.0 || !total_mass_val.is_finite() {
        return MassProperties {
            total_mass: Mass::new(0.0),
            center_of_mass: Vector3::zero(),
            moment_of_inertia_x: MomentOfInertia::new(0.0),
            moment_of_inertia_y: MomentOfInertia::new(0.0),
            moment_of_inertia_z: MomentOfInertia::new(0.0),
        };
    }

    let center_of_mass = sum_mr / total_mass_val;

    let mut ixx = 0.0;
    let mut iyy = 0.0;
    let mut izz = 0.0;

    for (m, pos) in component_masses {
        let dx = pos.0 - center_of_mass.0;
        let dy = pos.1 - center_of_mass.1;
        let dz = pos.2 - center_of_mass.2;

        let dx_sq = dx * dx;
        let dy_sq = dy * dy;
        let dz_sq = dz * dz;

        ixx += m * (dy_sq + dz_sq);
        iyy += m * (dx_sq + dz_sq);
        izz += m * (dx_sq + dy_sq);
    }

    MassProperties {
        total_mass: Mass::new(total_mass_val),
        center_of_mass,
        moment_of_inertia_x: MomentOfInertia::new(ixx),
        moment_of_inertia_y: MomentOfInertia::new(iyy),
        moment_of_inertia_z: MomentOfInertia::new(izz),
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
