use crate::chemistry::solvent::{solvent_properties_of, SolventProperties};
use crate::domain::Hydrosphere;
use crate::error::{DomainError, DomainResult};
use crate::units::constants::{
    DEFAULT_SOLUTE_MOLAR_MASS_KG, DEFAULT_VAN_T_HOFF_FACTOR, STANDARD_ATMOSPHERE_PRESSURE,
    UNIVERSAL_GAS_CONSTANT,
};
use crate::units::{
    Acceleration, Length, MolarMass, Pressure, Temperature, TemperatureGradient,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatterState {
    Solid,
    Liquid,
    Vapor,
    Supercritical,
}

pub fn boiling_point_clausius_clapeyron(
    pressure: Pressure,
    reference_boiling_point: Temperature,
    reference_pressure: Pressure,
    enthalpy_of_vaporization: f64,
) -> Temperature {
    let p = pressure.value();
    let p0 = reference_pressure.value();
    let t0 = reference_boiling_point.value();
    let delta_h = enthalpy_of_vaporization;

    if p <= 0.0
        || p0 <= 0.0
        || t0 <= 0.0
        || delta_h <= 0.0
        || !p.is_finite()
        || !p0.is_finite()
        || !t0.is_finite()
        || !delta_h.is_finite()
    {
        return Temperature::new(0.0);
    }

    let inv_t = (1.0 / t0) - (UNIVERSAL_GAS_CONSTANT / delta_h) * (p / p0).ln();

    if !inv_t.is_finite() || inv_t <= 0.0 {
        return Temperature::new(0.0);
    }

    Temperature::new(1.0 / inv_t)
}

pub fn dynamic_boiling_point(pressure: Pressure, properties: &SolventProperties) -> Temperature {
    let p = pressure.value();
    if p <= 0.0 || !p.is_finite() {
        return Temperature::new(0.0);
    }

    let t_boil = boiling_point_clausius_clapeyron(
        pressure,
        properties.normal_boiling_point,
        Pressure::new(STANDARD_ATMOSPHERE_PRESSURE),
        properties.enthalpy_of_vaporization,
    );

    if t_boil.value() <= 0.0 || t_boil.value() > properties.critical_temperature.value() {
        properties.critical_temperature
    } else {
        t_boil
    }
}

pub fn dynamic_boiling_point_of(pressure: Pressure, formula: &str) -> DomainResult<Temperature> {
    let props = solvent_properties_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
        field: "formula".to_string(),
        reason: format!("unknown solvent formula '{}'", formula),
    })?;

    Ok(dynamic_boiling_point(pressure, &props))
}

pub fn freezing_point_depression(
    solute_mass_fraction: f64,
    cryoscopic_constant: f64,
    solute_molar_mass_kg_per_mol: f64,
    van_t_hoff_factor: f64,
) -> Temperature {
    if !solute_mass_fraction.is_finite()
        || solute_mass_fraction <= 0.0
        || !cryoscopic_constant.is_finite()
        || cryoscopic_constant <= 0.0
        || !solute_molar_mass_kg_per_mol.is_finite()
        || solute_molar_mass_kg_per_mol <= 0.0
        || !van_t_hoff_factor.is_finite()
        || van_t_hoff_factor <= 0.0
    {
        return Temperature::new(0.0);
    }

    let w = solute_mass_fraction.clamp(0.0, 0.999);
    let molality = w / ((1.0 - w) * solute_molar_mass_kg_per_mol);
    let delta_t = cryoscopic_constant * molality * van_t_hoff_factor;

    if !delta_t.is_finite() || delta_t < 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(delta_t)
    }
}

pub fn depressed_freezing_point(
    normal_melting_point: Temperature,
    solute_mass_fraction: f64,
    cryoscopic_constant: f64,
    solute_molar_mass_kg_per_mol: f64,
    van_t_hoff_factor: f64,
) -> Temperature {
    let delta_t = freezing_point_depression(
        solute_mass_fraction,
        cryoscopic_constant,
        solute_molar_mass_kg_per_mol,
        van_t_hoff_factor,
    );

    let t_freeze = normal_melting_point.value() - delta_t.value();
    Temperature::new(t_freeze.max(0.0))
}

pub fn determine_matter_state(
    temperature: Temperature,
    pressure: Pressure,
    properties: &SolventProperties,
    solute_mass_fraction: f64,
) -> MatterState {
    let t = temperature.value();
    let p = pressure.value();

    if !t.is_finite() || t <= 0.0 {
        return MatterState::Solid;
    }

    if !p.is_finite() {
        return MatterState::Vapor;
    }

    if t >= properties.critical_temperature.value() && p >= properties.critical_pressure.value() {
        return MatterState::Supercritical;
    }

    if p < properties.triple_point_pressure.value() {
        let delta_h_subl = properties.enthalpy_of_vaporization + properties.enthalpy_of_fusion;
        let t_subl = boiling_point_clausius_clapeyron(
            pressure,
            properties.triple_point_temperature,
            properties.triple_point_pressure,
            delta_h_subl,
        );

        let t_subl_val = if t_subl.value() > 0.0 {
            t_subl.value()
        } else {
            properties.triple_point_temperature.value()
        };

        if t < t_subl_val {
            MatterState::Solid
        } else {
            MatterState::Vapor
        }
    } else {
        let t_freeze = depressed_freezing_point(
            properties.normal_melting_point,
            solute_mass_fraction,
            properties.cryoscopic_constant,
            DEFAULT_SOLUTE_MOLAR_MASS_KG,
            DEFAULT_VAN_T_HOFF_FACTOR,
        );

        let t_boil = dynamic_boiling_point(pressure, properties);

        if t < t_freeze.value() {
            MatterState::Solid
        } else if t > t_boil.value() {
            MatterState::Vapor
        } else {
            MatterState::Liquid
        }
    }
}

pub fn determine_matter_state_for_formula(
    temperature: Temperature,
    pressure: Pressure,
    formula: &str,
    solute_mass_fraction: f64,
) -> DomainResult<MatterState> {
    let props = solvent_properties_of(formula).ok_or_else(|| DomainError::InvalidInvariant {
        field: "formula".to_string(),
        reason: format!("unknown solvent formula '{}'", formula),
    })?;

    Ok(determine_matter_state(
        temperature,
        pressure,
        &props,
        solute_mass_fraction,
    ))
}

pub fn determine_hydrosphere_state(
    temperature: Temperature,
    pressure: Pressure,
    hydrosphere: &Hydrosphere,
) -> DomainResult<MatterState> {
    let props = hydrosphere.mean_solvent_properties()?;
    Ok(determine_matter_state(
        temperature,
        pressure,
        &props,
        hydrosphere.salinity_or_solute_mass_fraction(),
    ))
}

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
    let inv_td = (1.0 / t_surf) - (UNIVERSAL_GAS_CONSTANT / delta_h) * rh_clamped.ln();

    if !inv_td.is_finite() || inv_td <= 0.0 {
        return Temperature::new(0.0);
    }

    let td = 1.0 / inv_td;
    Temperature::new(td.min(t_surf))
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
    let g = gravity.value();
    let cp = if specific_heat_capacity > 0.0 && specific_heat_capacity.is_finite() {
        specific_heat_capacity
    } else {
        1000.0
    };

    if g <= 0.0 || !g.is_finite() {
        return TemperatureGradient::new(0.0);
    }

    let dry_rate = g / cp;

    let t = temperature.value();
    let p = pressure.value();

    if t <= 0.0 || p <= 0.0 || !t.is_finite() || !p.is_finite() {
        return TemperatureGradient::new(dry_rate);
    }

    let mv = solvent_molar_mass.value();
    let md = atmospheric_molar_mass.value();

    if mv <= 0.0 || md <= 0.0 || !mv.is_finite() || !md.is_finite() {
        return TemperatureGradient::new(dry_rate);
    }

    let delta_h = solvent_properties.enthalpy_of_vaporization;
    let t0 = solvent_properties.normal_boiling_point.value();
    let p0 = STANDARD_ATMOSPHERE_PRESSURE;

    if delta_h <= 0.0 || t0 <= 0.0 || !delta_h.is_finite() || !t0.is_finite() {
        return TemperatureGradient::new(dry_rate);
    }

    let lv_spec = delta_h / mv;
    let rd = UNIVERSAL_GAS_CONSTANT / md;
    let rv = UNIVERSAL_GAS_CONSTANT / mv;

    let exp_term = -(delta_h / UNIVERSAL_GAS_CONSTANT) * (1.0 / t - 1.0 / t0);
    let es = if exp_term > 50.0 {
        p * 0.99
    } else if exp_term < -50.0 {
        0.0
    } else {
        p0 * exp_term.exp()
    };

    let es_clamped = es.min(p * 0.99);
    if es_clamped <= 0.0 {
        return TemperatureGradient::new(dry_rate);
    }

    let rs = (mv / md) * (es_clamped / (p - es_clamped));
    if rs <= 0.0 || !rs.is_finite() {
        return TemperatureGradient::new(dry_rate);
    }

    let numerator = 1.0 + (lv_spec * rs) / (rd * t);
    let denominator = cp + (lv_spec * lv_spec * rs) / (rv * t * t);

    if denominator <= 0.0 || !denominator.is_finite() || !numerator.is_finite() {
        return TemperatureGradient::new(dry_rate);
    }

    let gamma_m = g * (numerator / denominator);
    if !gamma_m.is_finite() || gamma_m <= 0.0 {
        TemperatureGradient::new(dry_rate)
    } else {
        TemperatureGradient::new(gamma_m)
    }
}

pub fn lifting_condensation_level(
    surface_temperature: Temperature,
    dew_point: Temperature,
    dry_lapse_rate: TemperatureGradient,
    scale_height: Length,
    enthalpy_of_vaporization: f64,
) -> Length {
    let ts = surface_temperature.value();
    let td = dew_point.value();
    let gamma_d = dry_lapse_rate.value();
    let h = scale_height.value();
    let delta_h = enthalpy_of_vaporization;

    if ts <= 0.0 || td <= 0.0 || !ts.is_finite() || !td.is_finite() {
        return Length::new(0.0);
    }

    if td >= ts {
        return Length::new(0.0);
    }

    if gamma_d <= 0.0 || !gamma_d.is_finite() {
        return Length::new(0.0);
    }

    if h <= 0.0 || delta_h <= 0.0 || !h.is_finite() || !delta_h.is_finite() {
        let z = (ts - td) / gamma_d;
        return Length::new(z.max(0.0));
    }

    let a = gamma_d * (UNIVERSAL_GAS_CONSTANT / (delta_h * h));
    let b = (gamma_d / td) - (ts * UNIVERSAL_GAS_CONSTANT / (delta_h * h));
    let c = 1.0 - (ts / td);

    if a.abs() < 1e-18 {
        if b.abs() > 1e-18 {
            let z = -c / b;
            Length::new(z.max(0.0))
        } else {
            let z = (ts - td) / gamma_d;
            Length::new(z.max(0.0))
        }
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            let z = (ts - td) / gamma_d;
            Length::new(z.max(0.0))
        } else {
            let z = (-b + disc.sqrt()) / (2.0 * a);
            Length::new(z.max(0.0))
        }
    }
}

pub fn cloud_top_altitude(
    lcl_altitude: Length,
    surface_temperature: Temperature,
    surface_pressure: Pressure,
    environmental_lapse_rate: TemperatureGradient,
    moist_lapse_rate: TemperatureGradient,
    scale_height: Length,
    gravity: Acceleration,
    solvent_properties: &SolventProperties,
    solvent_molar_mass: MolarMass,
    atmospheric_molar_mass: MolarMass,
) -> Length {
    let z_lcl = lcl_altitude.value();
    let ts = surface_temperature.value();
    let ps = surface_pressure.value();
    let gamma_env = environmental_lapse_rate.value();
    let gamma_m_default = moist_lapse_rate.value();
    let h = scale_height.value();

    if z_lcl < 0.0 || !z_lcl.is_finite() || ts <= 0.0 || ps <= 0.0 || h <= 0.0 {
        return Length::new(0.0);
    }

    let t_lcl = ts - gamma_env * z_lcl;
    if t_lcl <= 0.0 {
        return Length::new(z_lcl);
    }

    let t_tropo = ts * 0.75;
    let dz = (h / 50.0).clamp(20.0, 200.0);
    let mut z = z_lcl;
    let mut t_cloud = t_lcl;

    for _ in 0..1000 {
        z += dz;
        let p_z = ps * (-z / h).exp();
        let local_gamma_m = moist_adiabatic_lapse_rate(
            gravity,
            1000.0,
            Temperature::new(t_cloud),
            Pressure::new(p_z),
            solvent_properties,
            solvent_molar_mass,
            atmospheric_molar_mass,
        )
        .value();

        let effective_gamma = if local_gamma_m > 0.0 && local_gamma_m.is_finite() {
            local_gamma_m
        } else if gamma_m_default > 0.0 && gamma_m_default.is_finite() {
            gamma_m_default
        } else {
            0.005
        };

        t_cloud -= effective_gamma * dz;
        let t_env = (ts - gamma_env * z).max(t_tropo);

        if t_cloud <= t_env || t_cloud <= 50.0 || p_z < ps * 1e-4 {
            break;
        }
    }

    Length::new(z.max(z_lcl))
}