use astronomicon_core::units::{Pressure, Temperature};

pub fn thermal_stress_from_gradient(
    thermal_expansion_coefficient: f64,
    youngs_modulus: Pressure,
    delta_temperature: Temperature,
) -> Pressure {
    let alpha = thermal_expansion_coefficient;
    let e = youngs_modulus.value();
    let dt = delta_temperature.value();

    if alpha <= 0.0
        || e <= 0.0
        || dt <= 0.0
        || !alpha.is_finite()
        || !e.is_finite()
        || !dt.is_finite()
    {
        return Pressure::new(0.0);
    }

    let stress = e * alpha * dt;
    if !stress.is_finite() || stress <= 0.0 {
        Pressure::new(0.0)
    } else {
        Pressure::new(stress)
    }
}

pub fn is_thermally_stressed_to_failure(
    total_stress: Pressure,
    effective_yield_strength: Pressure,
) -> bool {
    let s = total_stress.value();
    let sy = effective_yield_strength.value();

    if !s.is_finite() || !sy.is_finite() {
        return false;
    }

    s > sy
}

pub fn is_combined_stress_failure(
    thermal_stress: Pressure,
    mechanical_stress: Pressure,
    effective_yield_strength: Pressure,
) -> bool {
    is_thermally_stressed_to_failure(thermal_stress + mechanical_stress, effective_yield_strength)
}