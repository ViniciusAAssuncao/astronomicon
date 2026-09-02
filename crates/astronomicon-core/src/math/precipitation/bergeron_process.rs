use crate::chemistry::solvent::SolventProperties;
use crate::math::clouds::GlaciationState;
use crate::math::thermodynamics::{
    saturation_vapor_pressure, saturation_vapor_pressure_over_solid,
};
use crate::units::Temperature;

pub fn vapor_pressure_deficit_over_ice(
    temperature: Temperature,
    solvent_properties: &SolventProperties,
) -> f64 {
    let t = temperature.value();
    let t_triple = solvent_properties.triple_point_temperature.value();

    if !t.is_finite() || t <= 0.0 || t >= t_triple {
        return 0.0;
    }

    let p_liq = saturation_vapor_pressure(temperature, solvent_properties).value();
    let p_ice = saturation_vapor_pressure_over_solid(temperature, solvent_properties).value();

    if !p_liq.is_finite() || !p_ice.is_finite() || p_liq <= 0.0 || p_liq <= p_ice {
        return 0.0;
    }

    ((p_liq - p_ice) / p_liq).clamp(0.0, 1.0)
}

pub fn mixed_phase_coexistence_factor(ice_fraction: f64) -> f64 {
    let f = ice_fraction.clamp(0.0, 1.0);
    4.0 * f * (1.0 - f)
}

pub fn bergeron_enhancement_factor(
    glaciation_state: GlaciationState,
    ice_fraction: f64,
    temperature: Temperature,
    solvent_properties: &SolventProperties,
) -> f64 {
    if glaciation_state != GlaciationState::MixedPhase {
        return 1.0;
    }

    let deficit = vapor_pressure_deficit_over_ice(temperature, solvent_properties);
    if deficit <= 0.0 {
        return 1.0;
    }

    let coexistence = mixed_phase_coexistence_factor(ice_fraction);
    let enhancement = 1.0 + 5.0 * coexistence * deficit;

    enhancement.max(1.0)
}

pub fn apply_bergeron_enhancement(
    base_sedimentable_fraction: f64,
    enhancement_factor: f64,
) -> f64 {
    let base = base_sedimentable_fraction.clamp(0.0, 1.0);
    let factor = if enhancement_factor.is_finite() && enhancement_factor >= 1.0 {
        enhancement_factor
    } else {
        1.0
    };

    (base * factor).clamp(0.0, 1.0)
}