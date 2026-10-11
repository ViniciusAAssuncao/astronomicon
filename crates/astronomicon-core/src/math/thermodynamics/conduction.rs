use crate::units::{HeatFlux, Length, Luminosity, Temperature};

pub fn planar_thermal_resistance_per_unit_area(
    thickness: Length,
    thermal_conductivity: f64,
) -> f64 {
    let l = thickness.value();
    let k = thermal_conductivity;

    if l <= 0.0 || k <= 0.0 || !l.is_finite() || !k.is_finite() {
        0.0
    } else {
        l / k
    }
}

pub fn planar_thermal_resistance(
    thickness: Length,
    thermal_conductivity: f64,
    area_m2: f64,
) -> f64 {
    let l = thickness.value();
    let k = thermal_conductivity;
    let a = area_m2;

    if l <= 0.0 || k <= 0.0 || a <= 0.0 || !l.is_finite() || !k.is_finite() || !a.is_finite() {
        0.0
    } else {
        l / (k * a)
    }
}

pub fn equivalent_series_thermal_resistance(resistances: &[f64]) -> f64 {
    let mut total = 0.0;
    for &r in resistances {
        if r.is_finite() && r > 0.0 {
            total += r;
        }
    }
    total
}

pub fn equivalent_parallel_thermal_resistance(resistances: &[f64]) -> f64 {
    let mut sum_conductance = 0.0;
    for &r in resistances {
        if r.is_finite() && r > 0.0 {
            sum_conductance += 1.0 / r;
        }
    }
    if sum_conductance <= 0.0 || !sum_conductance.is_finite() {
        0.0
    } else {
        1.0 / sum_conductance
    }
}

pub fn conductive_heat_flux(
    thermal_conductivity: f64,
    thickness: Length,
    delta_temperature: Temperature,
) -> HeatFlux {
    let k = thermal_conductivity;
    let l = thickness.value();
    let dt = delta_temperature.value();

    if k <= 0.0 || l <= 0.0 || dt <= 0.0 || !k.is_finite() || !l.is_finite() || !dt.is_finite() {
        return HeatFlux::new(0.0);
    }

    let q = (k * dt) / l;
    if !q.is_finite() || q < 0.0 {
        HeatFlux::new(0.0)
    } else {
        HeatFlux::new(q)
    }
}

pub fn conductive_heat_transfer_rate(
    thermal_conductivity: f64,
    thickness: Length,
    area_m2: f64,
    delta_temperature: Temperature,
) -> Luminosity {
    let a = area_m2;
    if a <= 0.0 || !a.is_finite() {
        return Luminosity::new(0.0);
    }

    let flux = conductive_heat_flux(thermal_conductivity, thickness, delta_temperature);
    let power = flux.value() * a;

    if !power.is_finite() || power < 0.0 {
        Luminosity::new(0.0)
    } else {
        Luminosity::new(power)
    }
}

pub fn conductive_temperature_drop(
    heat_flux: HeatFlux,
    thickness: Length,
    thermal_conductivity: f64,
) -> Temperature {
    let q = heat_flux.value();
    let l = thickness.value();
    let k = thermal_conductivity;

    if q <= 0.0 || l <= 0.0 || k <= 0.0 || !q.is_finite() || !l.is_finite() || !k.is_finite() {
        return Temperature::new(0.0);
    }

    let dt = (q * l) / k;
    if !dt.is_finite() || dt < 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(dt)
    }
}

pub fn conductive_layer_thickness(
    thermal_conductivity: f64,
    delta_temperature: Temperature,
    heat_flux: HeatFlux,
) -> Length {
    let k = thermal_conductivity;
    let dt = delta_temperature.value();
    let q = heat_flux.value();

    if k <= 0.0 || dt <= 0.0 || !k.is_finite() || !dt.is_finite() {
        return Length::new(0.0);
    }

    if q <= 0.0 || !q.is_finite() {
        return Length::new(f64::INFINITY);
    }

    let l = (k * dt) / q;
    if !l.is_finite() || l < 0.0 {
        Length::new(0.0)
    } else {
        Length::new(l)
    }
}