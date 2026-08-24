use crate::chemistry::solvent::SolventProperties;
use crate::units::constants::{STANDARD_ATMOSPHERE_PRESSURE, UNIVERSAL_GAS_CONSTANT};
use crate::units::{MolarMass, Pressure, Temperature};

pub fn dew_point_temperature(
    surface_temperature: Temperature,
    relative_humidity: f64,
    enthalpy_of_vaporization: f64,
) -> Temperature {
    let t_surf = surface_temperature.value();
    let rh = relative_humidity;
    let delta_h = enthalpy_of_vaporization;

    if t_surf <= 0.0 || !t_surf.is_finite() || rh <= 0.0 || !rh.is_finite() {
        return Temperature::new(0.0);
    }

    if delta_h <= 0.0 || !delta_h.is_finite() {
        return surface_temperature;
    }

    let rh_clamped = rh.clamp(1e-6, 1.0);
    let inv_td = 1.0 / t_surf - (UNIVERSAL_GAS_CONSTANT / delta_h) * rh_clamped.ln();

    if !inv_td.is_finite() || inv_td <= 0.0 {
        return Temperature::new(0.0);
    }

    let td = 1.0 / inv_td;
    Temperature::new(td.min(t_surf))
}

pub fn saturation_vapor_pressure(
    temperature: Temperature,
    solvent_properties: &SolventProperties,
) -> Pressure {
    let t = temperature.value();
    let delta_h = solvent_properties.enthalpy_of_vaporization;
    let t0 = solvent_properties.normal_boiling_point.value();
    let p0 = STANDARD_ATMOSPHERE_PRESSURE;

    if t <= 0.0
        || !t.is_finite()
        || delta_h <= 0.0
        || !delta_h.is_finite()
        || t0 <= 0.0
        || !t0.is_finite()
    {
        return Pressure::new(0.0);
    }

    let exponent = -(delta_h / UNIVERSAL_GAS_CONSTANT) * (1.0 / t - 1.0 / t0);
    if exponent < -100.0 {
        Pressure::new(0.0)
    } else if exponent > 100.0 {
        Pressure::new(p0 * (100.0_f64).exp())
    } else {
        let p_sat = p0 * exponent.exp();
        if !p_sat.is_finite() || p_sat < 0.0 {
            Pressure::new(0.0)
        } else {
            Pressure::new(p_sat)
        }
    }
}

pub fn saturation_vapor_pressure_over_solid(
    temperature: Temperature,
    solvent_properties: &SolventProperties,
) -> Pressure {
    let t = temperature.value();
    let delta_h =
        solvent_properties.enthalpy_of_vaporization + solvent_properties.enthalpy_of_fusion;
    let t0 = solvent_properties.triple_point_temperature.value();
    let p0 = solvent_properties.triple_point_pressure.value();

    if t <= 0.0
        || !t.is_finite()
        || delta_h <= 0.0
        || !delta_h.is_finite()
        || t0 <= 0.0
        || !t0.is_finite()
        || p0 <= 0.0
        || !p0.is_finite()
    {
        return Pressure::new(0.0);
    }

    let exponent = -(delta_h / UNIVERSAL_GAS_CONSTANT) * (1.0 / t - 1.0 / t0);
    if exponent < -100.0 {
        Pressure::new(0.0)
    } else if exponent > 100.0 {
        Pressure::new(p0 * (100.0_f64).exp())
    } else {
        let p_sat = p0 * exponent.exp();
        if !p_sat.is_finite() || p_sat < 0.0 {
            Pressure::new(0.0)
        } else {
            Pressure::new(p_sat)
        }
    }
}

pub fn saturation_mixing_ratio(
    saturation_vapor_pressure: Pressure,
    ambient_pressure: Pressure,
    solvent_molar_mass: MolarMass,
    atmospheric_molar_mass: MolarMass,
) -> f64 {
    let es = saturation_vapor_pressure.value();
    let p = ambient_pressure.value();
    let mv = solvent_molar_mass.value();
    let md = atmospheric_molar_mass.value();

    if es <= 0.0
        || p <= 0.0
        || mv <= 0.0
        || md <= 0.0
        || !es.is_finite()
        || !p.is_finite()
        || !mv.is_finite()
        || !md.is_finite()
    {
        return 0.0;
    }

    let es_clamped = es.min(p * 0.99);
    let rs = (mv / md) * (es_clamped / (p - es_clamped));
    if !rs.is_finite() || rs <= 0.0 {
        0.0
    } else {
        rs
    }
}

pub fn mixing_ratio_from_relative_humidity(
    relative_humidity: f64,
    saturation_mixing_ratio: f64,
) -> f64 {
    let rh = relative_humidity.clamp(0.0, 1.0);
    if !saturation_mixing_ratio.is_finite() || saturation_mixing_ratio <= 0.0 {
        0.0
    } else {
        (rh * saturation_mixing_ratio).max(0.0)
    }
}