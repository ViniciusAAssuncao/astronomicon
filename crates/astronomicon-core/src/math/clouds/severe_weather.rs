use crate::units::{AngularVelocity, Length, MolarMass, SpecificEnergy, Speed, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StormMode {
    None,
    SingleCell,
    Multicell,
    Supercell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LightningPotential {
    None,
    Possible,
    Probable,
}

pub fn bulk_richardson_number(cape: SpecificEnergy, bulk_wind_shear: Speed) -> f64 {
    let c = cape.value();
    let du = bulk_wind_shear.value();

    if !c.is_finite() || !du.is_finite() || c <= 0.0 {
        return 0.0;
    }

    if du <= 0.0 {
        return f64::INFINITY;
    }

    let denom = 0.5 * du * du;
    if denom <= 0.0 {
        f64::INFINITY
    } else {
        c / denom
    }
}

pub fn classify_storm_mode(bulk_richardson_number: f64, cape: SpecificEnergy) -> StormMode {
    let c = cape.value();
    let brn = bulk_richardson_number;

    if !c.is_finite() || c < 100.0 || !brn.is_finite() || brn <= 0.0 {
        return StormMode::None;
    }

    if brn < 10.0 {
        StormMode::Multicell
    } else if brn <= 45.0 {
        StormMode::Supercell
    } else if brn <= 100.0 {
        StormMode::Multicell
    } else {
        StormMode::SingleCell
    }
}

pub fn evaluate_lightning_potential(
    cape: SpecificEnergy,
    mixed_phase_depth: Length,
    is_convective: bool,
) -> LightningPotential {
    if !is_convective {
        return LightningPotential::None;
    }

    let c = cape.value();
    let d_mp = mixed_phase_depth.value();

    if !c.is_finite() || !d_mp.is_finite() || c < 200.0 || d_mp < 400.0 {
        return LightningPotential::None;
    }

    if c >= 1000.0 && d_mp >= 1500.0 {
        LightningPotential::Probable
    } else if c >= 400.0 && d_mp >= 800.0 {
        LightningPotential::Possible
    } else {
        LightningPotential::None
    }
}

pub fn tropical_cyclone_potential_intensity(
    surface_temperature: Temperature,
    outflow_temperature: Temperature,
    enthalpy_of_vaporization: f64,
    solvent_molar_mass: MolarMass,
    surface_saturation_mixing_ratio: f64,
    boundary_layer_mixing_ratio: f64,
    exchange_coefficient_ratio_ck_over_cd: f64,
) -> Speed {
    let ts = surface_temperature.value();
    let to = outflow_temperature.value();
    let h_vap = enthalpy_of_vaporization;
    let m_solv = solvent_molar_mass.value();
    let r_sat = surface_saturation_mixing_ratio;
    let r_bl = boundary_layer_mixing_ratio;
    let ck_cd = exchange_coefficient_ratio_ck_over_cd;

    if !ts.is_finite()
        || !to.is_finite()
        || !h_vap.is_finite()
        || !m_solv.is_finite()
        || !r_sat.is_finite()
        || !r_bl.is_finite()
        || !ck_cd.is_finite()
        || to <= 0.0
        || ts <= to
        || m_solv <= 0.0
        || h_vap <= 0.0
        || ck_cd <= 0.0
    {
        return Speed::new(0.0);
    }

    let carnot_efficiency = (ts - to) / to;
    let lv_specific = h_vap / m_solv;
    let delta_r = (r_sat - r_bl).max(0.0);

    if delta_r <= 0.0 {
        return Speed::new(0.0);
    }

    let v_squared = ck_cd * carnot_efficiency * lv_specific * delta_r;
    if !v_squared.is_finite() || v_squared <= 0.0 {
        Speed::new(0.0)
    } else {
        Speed::new(v_squared.sqrt())
    }
}

pub fn is_cyclogenesis_favorable(
    ocean_coverage_fraction: f64,
    potential_intensity: Speed,
    coriolis_parameter: AngularVelocity,
) -> bool {
    let cov = ocean_coverage_fraction;
    let v_pot = potential_intensity.value();
    let f_cor = coriolis_parameter.value().abs();

    if !cov.is_finite() || !v_pot.is_finite() || !f_cor.is_finite() {
        return false;
    }

    let min_coriolis = 1.0e-5;
    let min_potential_speed = 15.0;

    cov > 0.0 && v_pot >= min_potential_speed && f_cor >= min_coriolis
}