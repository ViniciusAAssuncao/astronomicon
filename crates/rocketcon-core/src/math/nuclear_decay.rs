use crate::physics_reference::{radioisotope_properties_of, RadioisotopeType};
use astronomicon_core::units::{Duration, Frequency, Luminosity, Mass};

pub fn decay_constant(half_life: Duration) -> Frequency {
    let t_half = half_life.value();
    if t_half <= 0.0 || !t_half.is_finite() {
        Frequency::new(0.0)
    } else {
        Frequency::new(std::f64::consts::LN_2 / t_half)
    }
}

pub fn remaining_fraction(elapsed: Duration, half_life: Duration) -> f64 {
    let t_half = half_life.value();
    let t = elapsed.value();

    if t <= 0.0 {
        return 1.0;
    }

    if t_half <= 0.0 || !t_half.is_finite() || !t.is_finite() {
        return 0.0;
    }

    let lambda = std::f64::consts::LN_2 / t_half;
    let exponent = -lambda * t;

    if exponent < -700.0 {
        0.0
    } else {
        exponent.exp().clamp(0.0, 1.0)
    }
}

pub fn rtg_thermal_power(
    isotope: RadioisotopeType,
    fuel_mass: Mass,
    elapsed: Duration,
) -> Luminosity {
    let m = fuel_mass.value();
    if m <= 0.0 || !m.is_finite() {
        return Luminosity::new(0.0);
    }

    let props = radioisotope_properties_of(isotope);
    let frac = remaining_fraction(elapsed, props.half_life());
    let p_bol = props.specific_thermal_power_bol_w_per_kg() * m;
    let p_th = p_bol * frac;

    if !p_th.is_finite() || p_th <= 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(p_th)
    }
}

pub fn rtg_electrical_power(
    isotope: RadioisotopeType,
    fuel_mass: Mass,
    conversion_efficiency: f64,
    elapsed: Duration,
) -> Luminosity {
    let p_th = rtg_thermal_power(isotope, fuel_mass, elapsed);
    let eff = conversion_efficiency.clamp(0.0, 1.0);

    if !eff.is_finite() || eff <= 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(p_th.value() * eff)
    }
}

pub fn rtg_waste_heat(
    isotope: RadioisotopeType,
    fuel_mass: Mass,
    conversion_efficiency: f64,
    elapsed: Duration,
) -> Luminosity {
    let p_th = rtg_thermal_power(isotope, fuel_mass, elapsed);
    let p_el = rtg_electrical_power(isotope, fuel_mass, conversion_efficiency, elapsed);
    let diff = (p_th.value() - p_el.value()).max(0.0);

    if !diff.is_finite() {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(diff)
    }
}