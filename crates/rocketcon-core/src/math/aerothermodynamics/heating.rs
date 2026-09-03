use crate::constants::{
    DEFAULT_SUTTON_GRAVES_CONSTANT, MIN_LATERAL_AREA_M2, MIN_NOSE_RADIUS_M, STANTON_NUMBER_BASE,
    STANTON_NUMBER_MACH_COEFF, STANTON_NUMBER_MACH_EXPONENT,
};
use crate::domain::{ComponentRecord, VehicleComponentEntry};
use crate::math::aerothermodynamics::types::AerothermodynamicResults;
use astronomicon_core::units::{Density, HeatFlux, Length, Luminosity, Speed};
use std::f64::consts::PI;

pub fn stagnation_point_heat_flux_with_constant(
    nose_radius: Length,
    density: Density,
    speed: Speed,
    k_sg: f64,
) -> HeatFlux {
    let rn = nose_radius.value();
    let rho = density.value();
    let v = speed.value();

    if rn <= 0.0
        || rho <= 0.0
        || v <= 0.0
        || k_sg <= 0.0
        || !rn.is_finite()
        || !rho.is_finite()
        || !v.is_finite()
        || !k_sg.is_finite()
    {
        return HeatFlux::new(0.0);
    }

    let q = k_sg * (rho / rn).sqrt() * v.powi(3);
    if !q.is_finite() || q < 0.0 {
        HeatFlux::new(0.0)
    } else {
        HeatFlux::new(q)
    }
}

pub fn stagnation_point_heat_flux(
    nose_radius: Length,
    density: Density,
    speed: Speed,
) -> HeatFlux {
    stagnation_point_heat_flux_with_constant(
        nose_radius,
        density,
        speed,
        DEFAULT_SUTTON_GRAVES_CONSTANT,
    )
}

pub fn skin_friction_heat_flux(density: Density, speed: Speed, mach: f64) -> HeatFlux {
    let rho = density.value();
    let v = speed.value();

    if rho <= 0.0 || v <= 0.0 || !rho.is_finite() || !v.is_finite() {
        return HeatFlux::new(0.0);
    }

    let m = if mach.is_finite() && mach > 0.0 {
        mach
    } else {
        0.0
    };

    let stanton_number =
        STANTON_NUMBER_BASE / (1.0 + STANTON_NUMBER_MACH_COEFF * m * m).powf(STANTON_NUMBER_MACH_EXPONENT);
    let q = 0.5 * rho * v.powi(3) * stanton_number;

    if !q.is_finite() || q < 0.0 {
        HeatFlux::new(0.0)
    } else {
        HeatFlux::new(q)
    }
}

pub fn vehicle_geometry_thermal_properties(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
) -> (Length, f64, f64) {
    let mut max_diameter = 0.0;
    let mut total_lateral_area = 0.0;

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
    }

    let nose_r = (max_diameter * 0.5).max(MIN_NOSE_RADIUS_M);
    let nose_area = PI * nose_r * nose_r;
    let side_area = total_lateral_area.max(MIN_LATERAL_AREA_M2);

    (Length::new(nose_r), nose_area, side_area)
}

pub fn total_aerodynamic_heat_power(
    stagnation_flux: HeatFlux,
    nose_area_m2: f64,
    skin_flux: HeatFlux,
    side_area_m2: f64,
) -> Luminosity {
    let q_stag = stagnation_flux.value();
    let q_skin = skin_flux.value();
    let a_nose = if nose_area_m2.is_finite() && nose_area_m2 > 0.0 {
        nose_area_m2
    } else {
        0.0
    };
    let a_side = if side_area_m2.is_finite() && side_area_m2 > 0.0 {
        side_area_m2
    } else {
        0.0
    };

    let p_stag = if q_stag.is_finite() && q_stag > 0.0 {
        q_stag * a_nose
    } else {
        0.0
    };
    let p_skin = if q_skin.is_finite() && q_skin > 0.0 {
        q_skin * a_side
    } else {
        0.0
    };

    Luminosity::new((p_stag + p_skin).max(0.0))
}

pub fn evaluate_vehicle_aerothermodynamics(
    entries: &[(VehicleComponentEntry, ComponentRecord)],
    active_stages: &[u32],
    density: Density,
    relative_speed: Speed,
    mach: f64,
) -> AerothermodynamicResults {
    let (nose_radius, nose_area_m2, side_area_m2) =
        vehicle_geometry_thermal_properties(entries, active_stages);

    let stag_flux = stagnation_point_heat_flux(nose_radius, density, relative_speed);
    let skin_flux = skin_friction_heat_flux(density, relative_speed, mach);
    let total_power =
        total_aerodynamic_heat_power(stag_flux, nose_area_m2, skin_flux, side_area_m2);

    AerothermodynamicResults::new(
        stag_flux,
        skin_flux,
        nose_radius,
        nose_area_m2,
        side_area_m2,
        total_power,
    )
}