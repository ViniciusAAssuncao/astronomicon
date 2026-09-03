use crate::constants::{
    DEFAULT_HULL_SOLAR_ABSORPTIVITY, DEFAULT_SATELLITE_DRAG_COEFFICIENT_CD,
    DEFAULT_SOLAR_RADIATION_PRESSURE_CR,
};
use crate::domain::{ComponentDetails, ComponentRecord, VehicleComponentEntry};
use crate::math::rigid_body_shapes::{
    solid_cylinder_inertia_tensor, thin_shell_cylinder_inertia_tensor,
};
use astronomicon_core::units::{InertiaTensor, Length, Mass, Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleOpticalSurfaceProperties {
    pub total_surface_area_m2: f64,
    pub projected_cross_section_area_m2: f64,
    pub effective_srp_area_m2: f64,
    pub radiation_pressure_coefficient: f64,
    pub drag_area_product_m2: f64,
    pub effective_solar_absorptivity: f64,
    pub effective_reflectivity: f64,
}

impl VehicleOpticalSurfaceProperties {
    pub fn new(
        total_surface_area_m2: f64,
        projected_cross_section_area_m2: f64,
        effective_srp_area_m2: f64,
        radiation_pressure_coefficient: f64,
        drag_area_product_m2: f64,
        effective_solar_absorptivity: f64,
        effective_reflectivity: f64,
    ) -> Self {
        Self {
            total_surface_area_m2,
            projected_cross_section_area_m2,
            effective_srp_area_m2,
            radiation_pressure_coefficient,
            drag_area_product_m2,
            effective_solar_absorptivity,
            effective_reflectivity,
        }
    }

    pub fn default_simple(cross_section_area_m2: f64) -> Self {
        let a = cross_section_area_m2.max(0.01);
        let cr = DEFAULT_SOLAR_RADIATION_PRESSURE_CR;
        let cd = DEFAULT_SATELLITE_DRAG_COEFFICIENT_CD;
        let alpha = DEFAULT_HULL_SOLAR_ABSORPTIVITY;
        Self {
            total_surface_area_m2: a * 4.0,
            projected_cross_section_area_m2: a,
            effective_srp_area_m2: a,
            radiation_pressure_coefficient: cr,
            drag_area_product_m2: cd * a,
            effective_solar_absorptivity: alpha,
            effective_reflectivity: 1.0 - alpha,
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
            ComponentDetails::PropellantTank(_)
            | ComponentDetails::Hull(_)
            | ComponentDetails::HeatShield(_) => {
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

pub fn resolve_vehicle_optical_surface_properties(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
) -> VehicleOpticalSurfaceProperties {
    let mut max_diameter = 0.0f64;
    let mut total_lateral_area = 0.0f64;
    let mut weighted_cr_area = 0.0f64;
    let mut weighted_absorptivity_area = 0.0f64;
    let mut dedicated_srp_area = 0.0f64;

    for (entry, record) in entries {
        if !active_stages.contains(&entry.stage_index()) {
            continue;
        }

        let comp = record.component();
        let d = comp.diameter().value();
        let l = comp.length().value();

        if d.is_finite() && d > max_diameter {
            max_diameter = d;
        }
        if d.is_finite() && l.is_finite() && d > 0.0 && l > 0.0 {
            total_lateral_area += PI * d * l;
        }

        match record.details() {
            ComponentDetails::SolarPanel(spec) => {
                let area = spec.surface_area_m2();
                if area.is_finite() && area > 0.0 {
                    let alpha = spec.effective_solar_absorptivity();
                    let cr = 2.0 - alpha;
                    weighted_cr_area += cr * area;
                    weighted_absorptivity_area += alpha * area;
                    dedicated_srp_area += area;
                }
            }
            ComponentDetails::Radiator(spec) => {
                let area = spec.radiating_area_m2();
                if area.is_finite() && area > 0.0 {
                    let alpha = spec.solar_absorptivity();
                    let cr = 2.0 - alpha;
                    weighted_cr_area += cr * area;
                    weighted_absorptivity_area += alpha * area;
                    dedicated_srp_area += area;
                }
            }
            _ => {}
        }
    }

    let frontal_cross_section = if max_diameter > 0.0 {
        let r = max_diameter * 0.5;
        PI * r * r
    } else {
        0.01
    };

    let hull_area = total_lateral_area.max(frontal_cross_section);
    let hull_alpha = DEFAULT_HULL_SOLAR_ABSORPTIVITY;
    let hull_cr = 2.0 - hull_alpha;

    weighted_cr_area += hull_cr * frontal_cross_section;
    weighted_absorptivity_area += hull_alpha * frontal_cross_section;

    let total_srp_area = dedicated_srp_area + frontal_cross_section;
    let eff_cr = if total_srp_area > 0.0 {
        weighted_cr_area / total_srp_area
    } else {
        DEFAULT_SOLAR_RADIATION_PRESSURE_CR
    };
    let eff_alpha = if total_srp_area > 0.0 {
        weighted_absorptivity_area / total_srp_area
    } else {
        DEFAULT_HULL_SOLAR_ABSORPTIVITY
    };

    let total_surf = hull_area + dedicated_srp_area;
    let drag_area = DEFAULT_SATELLITE_DRAG_COEFFICIENT_CD * (frontal_cross_section + dedicated_srp_area * 0.5);

    VehicleOpticalSurfaceProperties::new(
        total_surf,
        frontal_cross_section,
        total_srp_area,
        eff_cr,
        drag_area,
        eff_alpha,
        (1.0 - eff_alpha).clamp(0.0, 1.0),
    )
}