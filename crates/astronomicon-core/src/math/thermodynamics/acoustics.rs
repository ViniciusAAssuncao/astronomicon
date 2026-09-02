use crate::units::{Speed, Temperature};

pub fn adiabatic_index_of_gas_mixture(cp: f64, specific_gas_constant: f64) -> f64 {
    if !cp.is_finite()
        || !specific_gas_constant.is_finite()
        || cp <= 0.0
        || specific_gas_constant <= 0.0
    {
        return 1.4;
    }
    let cv = cp - specific_gas_constant;
    if cv <= 0.0 || !cv.is_finite() {
        return 1.4;
    }
    let gamma = cp / cv;
    if !gamma.is_finite() || gamma <= 1.0 {
        1.4
    } else {
        gamma
    }
}

pub fn speed_of_sound(
    temperature: Temperature,
    specific_gas_constant: f64,
    adiabatic_index: f64,
) -> Speed {
    let t = temperature.value();
    let r = specific_gas_constant;
    let gamma = adiabatic_index;

    if t <= 0.0
        || r <= 0.0
        || gamma <= 0.0
        || !t.is_finite()
        || !r.is_finite()
        || !gamma.is_finite()
    {
        return Speed::new(0.0);
    }

    let a_sq = gamma * r * t;
    if a_sq <= 0.0 || !a_sq.is_finite() {
        Speed::new(0.0)
    } else {
        Speed::new(a_sq.sqrt())
    }
}
