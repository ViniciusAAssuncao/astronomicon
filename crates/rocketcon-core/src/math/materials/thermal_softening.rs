use crate::domain::MaterialClass;
use astronomicon_core::units::{Pressure, Temperature};

pub fn default_thermal_softening_exponent(material_class: MaterialClass) -> f64 {
    match material_class {
        MaterialClass::Metal => 2.0,
        MaterialClass::CompositeLaminate => 1.2,
        MaterialClass::Ceramic => 5.0,
        MaterialClass::AblativeComposite => 1.5,
        MaterialClass::Polymer => 1.0,
        MaterialClass::Exotic => 3.0,
    }
}

pub fn strength_retention_fraction(
    temperature: Temperature,
    melting_point: Temperature,
    exponent: f64,
) -> f64 {
    let t = temperature.value();
    let t_melt = melting_point.value();
    let n = if exponent.is_finite() && exponent > 0.0 {
        exponent
    } else {
        2.0
    };

    if t <= 0.0 {
        return 1.0;
    }

    if !t.is_finite() || !t_melt.is_finite() || t_melt <= 0.0 || t >= t_melt {
        return 0.0;
    }

    let homologous_ratio = (t / t_melt).clamp(0.0, 1.0);
    let retention = 1.0 - homologous_ratio.powf(n);
    retention.clamp(0.0, 1.0)
}

pub fn effective_yield_strength(
    base_yield_strength: Pressure,
    temperature: Temperature,
    melting_point: Temperature,
    exponent: f64,
) -> Pressure {
    let base = base_yield_strength.value();
    if base <= 0.0 || !base.is_finite() {
        return Pressure::new(0.0);
    }
    let retention = strength_retention_fraction(temperature, melting_point, exponent);
    Pressure::new(base * retention)
}

pub fn effective_ultimate_strength(
    base_ultimate_strength: Pressure,
    temperature: Temperature,
    melting_point: Temperature,
    exponent: f64,
) -> Pressure {
    let base = base_ultimate_strength.value();
    if base <= 0.0 || !base.is_finite() {
        return Pressure::new(0.0);
    }
    let retention = strength_retention_fraction(temperature, melting_point, exponent);
    Pressure::new(base * retention)
}

pub fn effective_ultimate_tensile_strength(
    base_ultimate_tensile_strength: Pressure,
    temperature: Temperature,
    melting_point: Temperature,
    exponent: f64,
) -> Pressure {
    effective_ultimate_strength(
        base_ultimate_tensile_strength,
        temperature,
        melting_point,
        exponent,
    )
}