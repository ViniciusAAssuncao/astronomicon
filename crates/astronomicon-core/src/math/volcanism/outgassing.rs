use crate::domain::PlanetKind;
use crate::units::constants::{
    CO2_HENRY_SOLUBILITY_COEFFICIENT, H2O_HENRY_SOLUBILITY_COEFFICIENT,
    SO2_HENRY_SOLUBILITY_COEFFICIENT,
};
use crate::units::{Acceleration, MassRate, Pressure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolcanicEruptionStyle {
    Effusive,
    Explosive,
    SubaqueousEffusive,
    Cryovolcanic,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VolcanicGasOutgassingRates {
    pub h2o: MassRate,
    pub co2: MassRate,
    pub so2: MassRate,
    pub h2s: MassRate,
    pub total: MassRate,
}

pub fn henry_solubility_h2o(surface_pressure: Pressure) -> f64 {
    let p = surface_pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return 0.0;
    }
    (H2O_HENRY_SOLUBILITY_COEFFICIENT * p.sqrt()).min(0.2)
}

pub fn henry_solubility_co2(surface_pressure: Pressure) -> f64 {
    let p = surface_pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return 0.0;
    }
    (CO2_HENRY_SOLUBILITY_COEFFICIENT * p).min(0.05)
}

pub fn henry_solubility_so2(surface_pressure: Pressure) -> f64 {
    let p = surface_pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return 0.0;
    }
    (SO2_HENRY_SOLUBILITY_COEFFICIENT * p.sqrt()).min(0.05)
}

pub fn exsolved_volatile_fraction(total_volatile_fraction: f64, solubility_limit: f64) -> f64 {
    if !total_volatile_fraction.is_finite() || total_volatile_fraction <= 0.0 {
        return 0.0;
    }
    (total_volatile_fraction - solubility_limit.max(0.0)).max(0.0)
}

pub fn classify_eruption_style(
    magma_viscosity_pa_s: f64,
    surface_pressure: Pressure,
    surface_gravity: Acceleration,
    exsolved_gas_mass_fraction: f64,
    is_subaqueous: bool,
    kind: PlanetKind,
    extrusion_rate: MassRate,
) -> VolcanicEruptionStyle {
    if extrusion_rate.value() <= 0.0 || !extrusion_rate.value().is_finite() {
        return VolcanicEruptionStyle::Inactive;
    }

    if matches!(
        kind,
        PlanetKind::IcyBody | PlanetKind::IceGiant | PlanetKind::DwarfPlanet
    ) {
        return VolcanicEruptionStyle::Cryovolcanic;
    }

    if is_subaqueous && surface_pressure.value() > 3.0e6 {
        return VolcanicEruptionStyle::SubaqueousEffusive;
    }

    let mu = magma_viscosity_pa_s;
    let p = surface_pressure.value();
    let g = surface_gravity.value();
    let x_g = exsolved_gas_mass_fraction.clamp(0.0, 1.0);

    if !mu.is_finite() || !p.is_finite() || !g.is_finite() {
        return VolcanicEruptionStyle::Effusive;
    }

    let fragmentation_index = (mu * x_g * g) / (p + 1.0e4);
    if fragmentation_index > 0.5 && x_g > 0.005 && mu > 1.0e3 {
        VolcanicEruptionStyle::Explosive
    } else {
        VolcanicEruptionStyle::Effusive
    }
}

pub fn volcanic_outgassing_fluxes(
    magma_extrusion_rate: MassRate,
    mantle_hydration: f64,
    c_o_ratio: f64,
    sulfur_mass_fraction: f64,
    surface_pressure: Pressure,
) -> VolcanicGasOutgassingRates {
    let m_dot = magma_extrusion_rate.value();
    if m_dot <= 0.0 || !m_dot.is_finite() {
        return VolcanicGasOutgassingRates {
            h2o: MassRate::new(0.0),
            co2: MassRate::new(0.0),
            so2: MassRate::new(0.0),
            h2s: MassRate::new(0.0),
            total: MassRate::new(0.0),
        };
    }

    let sol_h2o = henry_solubility_h2o(surface_pressure);
    let sol_co2 = henry_solubility_co2(surface_pressure);
    let sol_so2 = henry_solubility_so2(surface_pressure);

    let total_h2o = mantle_hydration.clamp(0.0, 0.1);
    let ex_h2o = exsolved_volatile_fraction(total_h2o, sol_h2o);

    let base_carbon_fraction = 0.002 * (c_o_ratio / 0.5).clamp(0.1, 5.0);
    let ex_c = exsolved_volatile_fraction(base_carbon_fraction, sol_co2);

    let total_s = sulfur_mass_fraction.clamp(0.0, 0.05);
    let ex_s = exsolved_volatile_fraction(total_s, sol_so2);

    let co2_fraction = if c_o_ratio > 0.8 {
        ex_c * 0.3
    } else {
        ex_c * 0.95
    };
    let (so2_fraction, h2s_fraction) = if c_o_ratio > 0.8 {
        (ex_s * 0.1, ex_s * 0.9)
    } else {
        (ex_s * 0.85, ex_s * 0.15)
    };

    let rate_h2o = m_dot * ex_h2o;
    let rate_co2 = m_dot * co2_fraction;
    let rate_so2 = m_dot * so2_fraction;
    let rate_h2s = m_dot * h2s_fraction;
    let total = rate_h2o + rate_co2 + rate_so2 + rate_h2s;

    VolcanicGasOutgassingRates {
        h2o: MassRate::new(rate_h2o),
        co2: MassRate::new(rate_co2),
        so2: MassRate::new(rate_so2),
        h2s: MassRate::new(rate_h2s),
        total: MassRate::new(total),
    }
}
