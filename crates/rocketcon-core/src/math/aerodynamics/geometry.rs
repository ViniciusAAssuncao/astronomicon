use crate::domain::{ComponentKind, ComponentRecord, VehicleComponentEntry};
use astronomicon_core::units::Vector3;
use std::f64::consts::PI;

pub fn vehicle_reference_cross_section_area(
    entries_and_records: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
) -> f64 {
    let mut max_diameter = 0.0;
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

pub fn resolve_center_of_pressure(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    mach: f64,
) -> Vector3 {
    let mut total_weight = 0.0;
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
    mach: f64,
) -> Vector3 {
    resolve_center_of_pressure(entries, active_stages, mach)
}