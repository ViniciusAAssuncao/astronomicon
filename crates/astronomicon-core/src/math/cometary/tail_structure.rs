use crate::units::constants::{ASTRONOMICAL_UNIT, SPEED_OF_LIGHT};
use crate::units::{Irradiance, Length, MassRate, Pressure, Speed};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CometaryTailStructure {
    pub ion_tail_length: Length,
    pub dust_tail_length: Length,
    pub ion_tail_drag_force_n: f64,
    pub dust_radiation_force_n: f64,
}

pub fn coma_radius(
    gas_production_rate: MassRate,
    expansion_speed: Speed,
    stellar_wind_dynamic_pressure: Pressure,
    irradiance: Irradiance,
) -> Length {
    let q = gas_production_rate.value();
    let v = expansion_speed.value();
    let p_sw = stellar_wind_dynamic_pressure.value();
    let f_irr = irradiance.value();

    if q <= 0.0 || v <= 0.0 || !q.is_finite() || !v.is_finite() {
        return Length::new(0.0);
    }

    let p_rad = if f_irr > 0.0 && f_irr.is_finite() {
        f_irr / SPEED_OF_LIGHT
    } else {
        0.0
    };

    let total_external_pressure = (p_sw + p_rad).max(1e-18);
    let r_coma = (q * v / (4.0 * PI * total_external_pressure)).sqrt();

    if !r_coma.is_finite() || r_coma <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(r_coma)
    }
}

pub fn cometary_tail_structure(
    gas_production_rate: MassRate,
    dust_to_gas_mass_ratio: f64,
    _expansion_speed: Speed,
    coma_radius: Length,
    stellar_wind_dynamic_pressure: Pressure,
    stellar_wind_speed: Speed,
    irradiance: Irradiance,
    heliocentric_distance: Length,
) -> CometaryTailStructure {
    let q_gas = gas_production_rate.value();
    let d_to_g = if dust_to_gas_mass_ratio.is_finite() && dust_to_gas_mass_ratio >= 0.0 {
        dust_to_gas_mass_ratio
    } else {
        1.0
    };
    let q_dust = q_gas * d_to_g;
    let v_wind = stellar_wind_speed.value();
    let p_sw = stellar_wind_dynamic_pressure.value();
    let f_irr = irradiance.value();
    let r_h = heliocentric_distance.value();
    let r_coma = coma_radius.value();

    if q_gas <= 0.0 || r_h <= 0.0 || !q_gas.is_finite() || !r_h.is_finite() {
        return CometaryTailStructure {
            ion_tail_length: Length::new(0.0),
            dust_tail_length: Length::new(0.0),
            ion_tail_drag_force_n: 0.0,
            dust_radiation_force_n: 0.0,
        };
    }

    let r_au = r_h / ASTRONOMICAL_UNIT;
    let tau_ion = 1.0e6 * r_au * r_au;
    let ion_length = (v_wind.max(1.0e3) * tau_ion).max(0.0);

    let area_coma = PI * r_coma * r_coma;
    let ion_drag_force = p_sw.max(0.0) * area_coma;

    let dust_grain_radius = 1.0e-6;
    let dust_grain_density = 1000.0;
    let p_rad = if f_irr > 0.0 && f_irr.is_finite() {
        f_irr / SPEED_OF_LIGHT
    } else {
        0.0
    };

    let q_pr = 1.0;
    let a_rad = (3.0 * p_rad * q_pr) / (4.0 * dust_grain_density * dust_grain_radius);
    let tau_dust = 1.5e6;
    let dust_length = 0.5 * a_rad * tau_dust * tau_dust;

    let total_dust_cross_section = if dust_grain_density > 0.0 && dust_grain_radius > 0.0 {
        (3.0 * q_dust * tau_dust) / (4.0 * dust_grain_density * dust_grain_radius)
    } else {
        0.0
    };
    let dust_radiation_force = p_rad * total_dust_cross_section;

    CometaryTailStructure {
        ion_tail_length: Length::new(ion_length),
        dust_tail_length: Length::new(dust_length),
        ion_tail_drag_force_n: ion_drag_force,
        dust_radiation_force_n: dust_radiation_force,
    }
}
