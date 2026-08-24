use crate::chemistry::geochemistry::condensation_temperature_50_of;
use crate::units::Temperature;

pub fn thermal_condensation_efficiency(
    condensation_temperature: Temperature,
    local_temperature: Temperature,
    transition_width: f64,
) -> f64 {
    let tc = condensation_temperature.value();
    let td = local_temperature.value();

    if !tc.is_finite() || !td.is_finite() || tc <= 0.0 {
        return 0.0;
    }

    if td <= 0.0 {
        return 1.0;
    }

    let width = if transition_width.is_finite() && transition_width > 0.0 {
        transition_width
    } else {
        50.0
    };

    let scale = width * 0.25;
    let arg = (td - tc) / scale;

    if arg >= 50.0 {
        0.0
    } else if arg <= -50.0 {
        1.0
    } else {
        let val = 1.0 / (1.0 + arg.exp());
        val.clamp(0.0, 1.0)
    }
}

pub fn condensation_fraction(
    element_symbol: &str,
    disk_temperature: Temperature,
    transition_width: f64,
) -> f64 {
    match condensation_temperature_50_of(element_symbol) {
        Some(tc) => thermal_condensation_efficiency(
            tc,
            disk_temperature,
            transition_width,
        ),
        None => 0.0,
    }
}