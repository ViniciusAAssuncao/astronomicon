use crate::math::clouds::cloud_fraction::cloud_band_altitudes;
use crate::math::clouds::cloud_water_content::ice_fraction_at_altitude;
use crate::units::{Length, SpecificEnergy, TemperatureGradient};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AtmosphericStability {
    AbsolutelyUnstable,
    ConditionallyUnstable,
    AbsolutelyStable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloudMorphology {
    Stratiform,
    Convective,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GlaciationState {
    Liquid,
    MixedPhase,
    Glaciated,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudClassification {
    pub stability: AtmosphericStability,
    pub morphology: CloudMorphology,
    pub is_deep_convection: bool,
    pub low_glaciation: GlaciationState,
    pub mid_glaciation: GlaciationState,
    pub high_glaciation: GlaciationState,
}

pub fn classify_atmospheric_stability(
    environmental_lapse_rate: TemperatureGradient,
    dry_adiabatic_lapse_rate: TemperatureGradient,
    moist_adiabatic_lapse_rate: TemperatureGradient,
) -> AtmosphericStability {
    let gamma_env = environmental_lapse_rate.value();
    let gamma_d = dry_adiabatic_lapse_rate.value();
    let gamma_m = moist_adiabatic_lapse_rate.value();

    if !gamma_env.is_finite() || !gamma_d.is_finite() || !gamma_m.is_finite() {
        return AtmosphericStability::AbsolutelyStable;
    }

    if gamma_env > gamma_d {
        AtmosphericStability::AbsolutelyUnstable
    } else if gamma_env >= gamma_m {
        AtmosphericStability::ConditionallyUnstable
    } else {
        AtmosphericStability::AbsolutelyStable
    }
}

pub fn classify_cloud_morphology(
    stability: AtmosphericStability,
    cape: SpecificEnergy,
) -> CloudMorphology {
    match stability {
        AtmosphericStability::AbsolutelyUnstable => CloudMorphology::Convective,
        AtmosphericStability::ConditionallyUnstable => {
            if cape.value() > 0.0 {
                CloudMorphology::Convective
            } else {
                CloudMorphology::Stratiform
            }
        }
        AtmosphericStability::AbsolutelyStable => CloudMorphology::Stratiform,
    }
}

pub fn is_deep_convective_cloud(
    morphology: CloudMorphology,
    cloud_top_altitude: Length,
    tropopause_altitude: Length,
    cape: SpecificEnergy,
) -> bool {
    if morphology != CloudMorphology::Convective {
        return false;
    }

    let z_top = cloud_top_altitude.value();
    let z_tropo = tropopause_altitude.value();
    let c = cape.value();

    if !z_top.is_finite() || !z_tropo.is_finite() || !c.is_finite() || z_tropo <= 0.0 {
        return false;
    }

    c >= 400.0 && z_top >= 0.75 * z_tropo
}

pub fn glaciation_state_from_ice_fraction(ice_fraction: f64) -> GlaciationState {
    let f = ice_fraction.clamp(0.0, 1.0);
    if f < 0.10 {
        GlaciationState::Liquid
    } else if f > 0.90 {
        GlaciationState::Glaciated
    } else {
        GlaciationState::MixedPhase
    }
}

pub fn classify_cloud_system(
    environmental_lapse_rate: TemperatureGradient,
    dry_adiabatic_lapse_rate: TemperatureGradient,
    moist_adiabatic_lapse_rate: TemperatureGradient,
    cape: SpecificEnergy,
    cloud_top_altitude: Length,
    tropopause_altitude: Length,
    freezing_level_altitude: Length,
) -> CloudClassification {
    let stability = classify_atmospheric_stability(
        environmental_lapse_rate,
        dry_adiabatic_lapse_rate,
        moist_adiabatic_lapse_rate,
    );

    let morphology = classify_cloud_morphology(stability, cape);
    let is_deep = is_deep_convective_cloud(
        morphology,
        cloud_top_altitude,
        tropopause_altitude,
        cape,
    );

    let (z0, z_low, z_mid, z_high) = cloud_band_altitudes(tropopause_altitude);
    let z_low_mid = Length::new(0.5 * (z0.value() + z_low.value()));
    let z_mid_mid = Length::new(0.5 * (z_low.value() + z_mid.value()));
    let z_high_mid = Length::new(0.5 * (z_mid.value() + z_high.value()));

    let low_ice = ice_fraction_at_altitude(z_low_mid, freezing_level_altitude);
    let mid_ice = ice_fraction_at_altitude(z_mid_mid, freezing_level_altitude);
    let high_ice = ice_fraction_at_altitude(z_high_mid, freezing_level_altitude);

    CloudClassification {
        stability,
        morphology,
        is_deep_convection: is_deep,
        low_glaciation: glaciation_state_from_ice_fraction(low_ice),
        mid_glaciation: glaciation_state_from_ice_fraction(mid_ice),
        high_glaciation: glaciation_state_from_ice_fraction(high_ice),
    }
}