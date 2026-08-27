use crate::error::{DomainError, DomainResult};
use crate::units::Temperature;
use serde::{Deserialize, Serialize};

pub const STANDARD_ENTHALPY_H2_G: f64 = 0.0;
pub const STANDARD_GIBBS_H2_G: f64 = 0.0;

pub const STANDARD_ENTHALPY_O2_G: f64 = 0.0;
pub const STANDARD_GIBBS_O2_G: f64 = 0.0;

pub const STANDARD_ENTHALPY_H2O_L: f64 = -285_830.0;
pub const STANDARD_GIBBS_H2O_L: f64 = -237_130.0;

pub const STANDARD_ENTHALPY_H2O_G: f64 = -241_820.0;
pub const STANDARD_GIBBS_H2O_G: f64 = -228_570.0;

pub const STANDARD_ENTHALPY_CO2_G: f64 = -393_510.0;
pub const STANDARD_GIBBS_CO2_G: f64 = -394_360.0;

pub const STANDARD_ENTHALPY_CO_G: f64 = -110_530.0;
pub const STANDARD_GIBBS_CO_G: f64 = -137_170.0;

pub const STANDARD_ENTHALPY_CH4_G: f64 = -74_810.0;
pub const STANDARD_GIBBS_CH4_G: f64 = -50_720.0;

pub const STANDARD_ENTHALPY_H2S_G: f64 = -20_600.0;
pub const STANDARD_GIBBS_H2S_G: f64 = -33_400.0;

pub const STANDARD_ENTHALPY_H2S_AQ: f64 = -39_700.0;
pub const STANDARD_GIBBS_H2S_AQ: f64 = -27_830.0;

pub const STANDARD_ENTHALPY_SO4_AQ: f64 = -909_270.0;
pub const STANDARD_GIBBS_SO4_AQ: f64 = -744_530.0;

pub const STANDARD_ENTHALPY_SO2_G: f64 = -296_830.0;
pub const STANDARD_GIBBS_SO2_G: f64 = -300_190.0;

pub const STANDARD_ENTHALPY_NH3_G: f64 = -46_110.0;
pub const STANDARD_GIBBS_NH3_G: f64 = -16_450.0;

pub const STANDARD_ENTHALPY_FE2_AQ: f64 = -89_100.0;
pub const STANDARD_GIBBS_FE2_AQ: f64 = -78_900.0;

pub const STANDARD_ENTHALPY_FE3_AQ: f64 = -48_500.0;
pub const STANDARD_GIBBS_FE3_AQ: f64 = -4_700.0;

pub const STANDARD_ENTHALPY_H_AQ: f64 = 0.0;
pub const STANDARD_GIBBS_H_AQ: f64 = 0.0;

pub const STANDARD_ENTHALPY_S_S: f64 = 0.0;
pub const STANDARD_GIBBS_S_S: f64 = 0.0;

pub const STANDARD_ENTHALPY_NO3_AQ: f64 = -205_000.0;
pub const STANDARD_GIBBS_NO3_AQ: f64 = -108_740.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermodynamicProperties {
    pub enthalpy_of_formation: f64,
    pub gibbs_free_energy_of_formation: f64,
}

impl ThermodynamicProperties {
    pub const fn new(enthalpy_of_formation: f64, gibbs_free_energy_of_formation: f64) -> Self {
        Self {
            enthalpy_of_formation,
            gibbs_free_energy_of_formation,
        }
    }
}

pub fn thermodynamic_properties_of(species: &str) -> Option<ThermodynamicProperties> {
    match species {
        "H2" | "H2(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_H2_G,
            STANDARD_GIBBS_H2_G,
        )),
        "O2" | "O2(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_O2_G,
            STANDARD_GIBBS_O2_G,
        )),
        "H2O" | "H2O(l)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_H2O_L,
            STANDARD_GIBBS_H2O_L,
        )),
        "H2O(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_H2O_G,
            STANDARD_GIBBS_H2O_G,
        )),
        "CO2" | "CO2(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_CO2_G,
            STANDARD_GIBBS_CO2_G,
        )),
        "CO" | "CO(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_CO_G,
            STANDARD_GIBBS_CO_G,
        )),
        "CH4" | "CH4(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_CH4_G,
            STANDARD_GIBBS_CH4_G,
        )),
        "H2S" | "H2S(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_H2S_G,
            STANDARD_GIBBS_H2S_G,
        )),
        "H2S(aq)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_H2S_AQ,
            STANDARD_GIBBS_H2S_AQ,
        )),
        "SO4_2-" | "SO4^2-" | "SO4^2-(aq)" | "SO4_2-(aq)" | "SO4(aq)" => {
            Some(ThermodynamicProperties::new(
                STANDARD_ENTHALPY_SO4_AQ,
                STANDARD_GIBBS_SO4_AQ,
            ))
        }
        "SO2" | "SO2(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_SO2_G,
            STANDARD_GIBBS_SO2_G,
        )),
        "NH3" | "NH3(g)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_NH3_G,
            STANDARD_GIBBS_NH3_G,
        )),
        "Fe2+" | "Fe^2+" | "Fe2+(aq)" | "Fe^2+(aq)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_FE2_AQ,
            STANDARD_GIBBS_FE2_AQ,
        )),
        "Fe3+" | "Fe^3+" | "Fe3+(aq)" | "Fe^3+(aq)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_FE3_AQ,
            STANDARD_GIBBS_FE3_AQ,
        )),
        "H+" | "H+(aq)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_H_AQ,
            STANDARD_GIBBS_H_AQ,
        )),
        "S" | "S(s)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_S_S,
            STANDARD_GIBBS_S_S,
        )),
        "NO3-" | "NO3^-" | "NO3^-(aq)" | "NO3-(aq)" => Some(ThermodynamicProperties::new(
            STANDARD_ENTHALPY_NO3_AQ,
            STANDARD_GIBBS_NO3_AQ,
        )),
        _ => None,
    }
}

pub fn standard_enthalpy_of_formation(species: &str) -> Option<f64> {
    thermodynamic_properties_of(species).map(|p| p.enthalpy_of_formation)
}

pub fn standard_gibbs_of_formation(species: &str) -> Option<f64> {
    thermodynamic_properties_of(species).map(|p| p.gibbs_free_energy_of_formation)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReactionThermodynamics {
    pub delta_h_standard: f64,
    pub delta_g_standard: f64,
    pub delta_s_standard: f64,
    pub delta_g_at_temperature: f64,
    pub is_spontaneous: bool,
}

pub fn reaction_standard_enthalpy(
    reactants: &[(&str, f64)],
    products: &[(&str, f64)],
) -> DomainResult<f64> {
    let mut sum = 0.0;
    for &(prod, coeff) in products {
        let props =
            thermodynamic_properties_of(prod).ok_or_else(|| DomainError::InvalidInvariant {
                field: "products".to_string(),
                reason: format!("unknown thermochemical species '{}'", prod),
            })?;
        sum += props.enthalpy_of_formation * coeff;
    }
    for &(reac, coeff) in reactants {
        let props =
            thermodynamic_properties_of(reac).ok_or_else(|| DomainError::InvalidInvariant {
                field: "reactants".to_string(),
                reason: format!("unknown thermochemical species '{}'", reac),
            })?;
        sum -= props.enthalpy_of_formation * coeff;
    }
    Ok(sum)
}

pub fn reaction_standard_gibbs_energy(
    reactants: &[(&str, f64)],
    products: &[(&str, f64)],
) -> DomainResult<f64> {
    let mut sum = 0.0;
    for &(prod, coeff) in products {
        let props =
            thermodynamic_properties_of(prod).ok_or_else(|| DomainError::InvalidInvariant {
                field: "products".to_string(),
                reason: format!("unknown thermochemical species '{}'", prod),
            })?;
        sum += props.gibbs_free_energy_of_formation * coeff;
    }
    for &(reac, coeff) in reactants {
        let props =
            thermodynamic_properties_of(reac).ok_or_else(|| DomainError::InvalidInvariant {
                field: "reactants".to_string(),
                reason: format!("unknown thermochemical species '{}'", reac),
            })?;
        sum -= props.gibbs_free_energy_of_formation * coeff;
    }
    Ok(sum)
}

pub fn reaction_standard_entropy(delta_h_standard: f64, delta_g_standard: f64) -> f64 {
    let t_ref = 298.15;
    (delta_h_standard - delta_g_standard) / t_ref
}

pub fn gibbs_helmholtz_temperature_correction(
    delta_h_standard: f64,
    delta_g_standard: f64,
    temperature: Temperature,
) -> f64 {
    let t = temperature.value();
    if t <= 0.0 || !t.is_finite() {
        return delta_g_standard;
    }
    let delta_s = reaction_standard_entropy(delta_h_standard, delta_g_standard);
    delta_h_standard - t * delta_s
}

pub fn evaluate_reaction(
    reactants: &[(&str, f64)],
    products: &[(&str, f64)],
    temperature: Temperature,
) -> DomainResult<ReactionThermodynamics> {
    let delta_h = reaction_standard_enthalpy(reactants, products)?;
    let delta_g_std = reaction_standard_gibbs_energy(reactants, products)?;
    let delta_s = reaction_standard_entropy(delta_h, delta_g_std);
    let delta_g_t = gibbs_helmholtz_temperature_correction(delta_h, delta_g_std, temperature);
    let is_spontaneous = delta_g_t < 0.0;

    Ok(ReactionThermodynamics {
        delta_h_standard: delta_h,
        delta_g_standard: delta_g_std,
        delta_s_standard: delta_s,
        delta_g_at_temperature: delta_g_t,
        is_spontaneous,
    })
}

pub fn hydrogen_sulfide_oxidation(temperature: Temperature) -> ReactionThermodynamics {
    evaluate_reaction(
        &[("H2S(aq)", 1.0), ("O2(g)", 2.0)],
        &[("SO4^2-(aq)", 1.0), ("H+(aq)", 2.0)],
        temperature,
    )
    .expect("valid thermodynamic reaction parameters")
}

pub fn hydrogen_sulfide_partial_oxidation(temperature: Temperature) -> ReactionThermodynamics {
    evaluate_reaction(
        &[("H2S(aq)", 1.0), ("O2(g)", 0.5)],
        &[("S(s)", 1.0), ("H2O(l)", 1.0)],
        temperature,
    )
    .expect("valid thermodynamic reaction parameters")
}

pub fn methanogenesis(temperature: Temperature) -> ReactionThermodynamics {
    evaluate_reaction(
        &[("CO2(g)", 1.0), ("H2(g)", 4.0)],
        &[("CH4(g)", 1.0), ("H2O(l)", 2.0)],
        temperature,
    )
    .expect("valid thermodynamic reaction parameters")
}

pub fn methanotrophy(temperature: Temperature) -> ReactionThermodynamics {
    evaluate_reaction(
        &[("CH4(g)", 1.0), ("O2(g)", 2.0)],
        &[("CO2(g)", 1.0), ("H2O(l)", 2.0)],
        temperature,
    )
    .expect("valid thermodynamic reaction parameters")
}

pub fn iron_oxidation(temperature: Temperature) -> ReactionThermodynamics {
    evaluate_reaction(
        &[("Fe2+(aq)", 4.0), ("O2(g)", 1.0), ("H+(aq)", 4.0)],
        &[("Fe3+(aq)", 4.0), ("H2O(l)", 2.0)],
        temperature,
    )
    .expect("valid thermodynamic reaction parameters")
}
