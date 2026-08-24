use crate::chemistry::composition_mean::{
    composition_weighted_mean_or_zero, normalize_composition_fractions,
};
use crate::chemistry::molecular_formula;
use crate::chemistry::periodic_table::{atomic_number, atomic_weight};
use crate::error::DomainResult;
use crate::units::constants::UNIVERSAL_GAS_CONSTANT;
use crate::units::{MassAttenuationCoefficient, MolarMass};

pub fn molar_mass_of(formula: &str) -> DomainResult<MolarMass> {
    let parsed = molecular_formula::parse(formula)?;
    let mut total_g_per_mol = 0.0;

    for (symbol, count) in parsed {
        if let Some(weight) = atomic_weight(&symbol) {
            total_g_per_mol += weight * (count as f64);
        }
    }

    Ok(MolarMass::new(total_g_per_mol * 0.001))
}

pub fn mean_molar_mass(composition: &[(String, f64)]) -> DomainResult<MolarMass> {
    let mean_kg_per_mol = composition_weighted_mean_or_zero(composition, |formula| {
        molar_mass_of(formula).map(|m| m.value())
    })?;

    Ok(MolarMass::new(mean_kg_per_mol))
}

pub fn molar_heat_capacity_of(formula: &str) -> DomainResult<f64> {
    let parsed = molecular_formula::parse(formula)?;
    let total_atoms: u32 = parsed.iter().map(|(_, count)| count).sum();

    let c_p_molar = match total_atoms {
        0 => 0.0,
        1 => 2.5 * UNIVERSAL_GAS_CONSTANT,
        2 => 3.5 * UNIVERSAL_GAS_CONSTANT,
        _ => match formula {
            "CO2" | "SO2" | "N2O" | "NO2" | "O3" => 4.5 * UNIVERSAL_GAS_CONSTANT,
            "H2O" => 4.0 * UNIVERSAL_GAS_CONSTANT,
            "CH4" => 4.3 * UNIVERSAL_GAS_CONSTANT,
            _ => (3.0 + 0.5 * (total_atoms as f64)) * UNIVERSAL_GAS_CONSTANT,
        },
    };

    Ok(c_p_molar)
}

pub fn specific_heat_capacity_of(formula: &str) -> DomainResult<f64> {
    let molar_cp = molar_heat_capacity_of(formula)?;
    let molar_mass = molar_mass_of(formula)?.value();

    if molar_mass <= 0.0 {
        return Ok(0.0);
    }

    Ok(molar_cp / molar_mass)
}

pub fn mean_specific_heat_capacity(composition: &[(String, f64)]) -> DomainResult<f64> {
    let fractions = match normalize_composition_fractions(composition) {
        Some(f) => f,
        None => return Ok(0.0),
    };

    let mut mean_molar_cp = 0.0;
    let mut mean_molar_mass_val = 0.0;

    for (formula, fraction) in fractions {
        let molar_cp = molar_heat_capacity_of(formula)?;
        let molar_mass = molar_mass_of(formula)?.value();

        mean_molar_cp += molar_cp * fraction;
        mean_molar_mass_val += molar_mass * fraction;
    }

    if mean_molar_mass_val <= 0.0 {
        return Ok(0.0);
    }

    Ok(mean_molar_cp / mean_molar_mass_val)
}

pub fn mass_attenuation_coefficient_of(formula: &str) -> DomainResult<MassAttenuationCoefficient> {
    let parsed = molecular_formula::parse(formula)?;
    let mut total_electrons = 0.0;
    let mut total_atomic_mass = 0.0;
    let mut total_z_sq = 0.0;

    for (symbol, count) in parsed {
        let count_f = count as f64;
        if let Some(weight) = atomic_weight(&symbol) {
            total_atomic_mass += weight * count_f;
        }
        if let Some(z) = atomic_number(&symbol) {
            let z_f = z as f64;
            total_electrons += z_f * count_f;
            total_z_sq += z_f * z_f * count_f;
        }
    }

    if total_atomic_mass <= 0.0 || total_electrons <= 0.0 {
        return Ok(MassAttenuationCoefficient::new(0.0));
    }

    let z_eff = total_z_sq / total_electrons;
    let z_over_a = total_electrons / total_atomic_mass;
    let base_mu = 0.001;
    let coeff = base_mu * z_over_a * 2.0 * (1.0 + 0.02 * z_eff);

    Ok(MassAttenuationCoefficient::new(coeff))
}

pub fn mean_mass_attenuation_coefficient(
    composition: &[(String, f64)],
) -> DomainResult<MassAttenuationCoefficient> {
    let mean_val = composition_weighted_mean_or_zero(composition, |formula| {
        mass_attenuation_coefficient_of(formula).map(|c| c.value())
    })?;

    Ok(MassAttenuationCoefficient::new(mean_val))
}
