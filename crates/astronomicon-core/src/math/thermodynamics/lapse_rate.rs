use crate::chemistry::solvent::SolventProperties;
use crate::math::thermodynamics::vapor_pressure::{
    saturation_mixing_ratio, saturation_vapor_pressure,
};
use crate::units::constants::UNIVERSAL_GAS_CONSTANT;
use crate::units::{
    Acceleration, Length, MolarMass, Pressure, Temperature, TemperatureGradient,
};

pub fn dry_adiabatic_lapse_rate(
    gravity: Acceleration,
    specific_heat_capacity: f64,
) -> TemperatureGradient {
    let g = gravity.value();
    let cp = specific_heat_capacity;
    if g <= 0.0 || cp <= 0.0 || !g.is_finite() || !cp.is_finite() {
        TemperatureGradient::new(0.0)
    } else {
        TemperatureGradient::new(g / cp)
    }
}

pub fn moist_adiabatic_lapse_rate(
    gravity: Acceleration,
    specific_heat_capacity: f64,
    temperature: Temperature,
    pressure: Pressure,
    solvent_properties: &SolventProperties,
    solvent_molar_mass: MolarMass,
    atmospheric_molar_mass: MolarMass,
) -> TemperatureGradient {
    let dry_rate = dry_adiabatic_lapse_rate(gravity, specific_heat_capacity);
    let g = gravity.value();
    let cp = if specific_heat_capacity > 0.0 && specific_heat_capacity.is_finite() {
        specific_heat_capacity
    } else {
        1000.0
    };

    if g <= 0.0 || !g.is_finite() {
        return TemperatureGradient::new(0.0);
    }

    let t = temperature.value();
    let p = pressure.value();

    if t <= 0.0 || p <= 0.0 || !t.is_finite() || !p.is_finite() {
        return dry_rate;
    }

    let mv = solvent_molar_mass.value();
    let md = atmospheric_molar_mass.value();

    if mv <= 0.0 || md <= 0.0 || !mv.is_finite() || !md.is_finite() {
        return dry_rate;
    }

    let delta_h = solvent_properties.enthalpy_of_vaporization;
    if delta_h <= 0.0 || !delta_h.is_finite() {
        return dry_rate;
    }

    let p_sat = saturation_vapor_pressure(temperature, solvent_properties);
    let rs = saturation_mixing_ratio(p_sat, pressure, solvent_molar_mass, atmospheric_molar_mass);
    if rs <= 0.0 {
        return dry_rate;
    }

    let lv_spec = delta_h / mv;
    let rd = UNIVERSAL_GAS_CONSTANT / md;
    let rv = UNIVERSAL_GAS_CONSTANT / mv;

    let numerator = 1.0 + (lv_spec * rs) / (rd * t);
    let denominator = cp + (lv_spec * lv_spec * rs) / (rv * t * t);

    if denominator <= 0.0 || !denominator.is_finite() || !numerator.is_finite() {
        return dry_rate;
    }

    let gamma_m = g * (numerator / denominator);
    if !gamma_m.is_finite() || gamma_m <= 0.0 {
        dry_rate
    } else {
        TemperatureGradient::new(gamma_m)
    }
}

pub fn grey_atmosphere_skin_temperature(
    radiative_equilibrium_temperature: Temperature,
) -> Temperature {
    let t_eq = radiative_equilibrium_temperature.value();
    if t_eq <= 0.0 || !t_eq.is_finite() {
        return Temperature::new(0.0);
    }
    let factor = (0.5_f64).powf(0.25);
    Temperature::new(t_eq * factor)
}

pub fn tropopause_altitude(
    surface_temperature: Temperature,
    skin_temperature: Temperature,
    environmental_lapse_rate: TemperatureGradient,
) -> Length {
    let t_surf = surface_temperature.value();
    let t_skin = skin_temperature.value();
    let gamma = environmental_lapse_rate.value();

    if !t_surf.is_finite() || !t_skin.is_finite() || t_surf <= 0.0 || t_skin <= 0.0 {
        return Length::new(0.0);
    }

    if gamma <= 0.0 || !gamma.is_finite() {
        return Length::new(f64::INFINITY);
    }

    if t_surf <= t_skin {
        return Length::new(0.0);
    }

    let z = (t_surf - t_skin) / gamma;
    if !z.is_finite() || z <= 0.0 {
        Length::new(0.0)
    } else {
        Length::new(z)
    }
}