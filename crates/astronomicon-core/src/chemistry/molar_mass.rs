use crate::chemistry::molecular_formula;
use crate::chemistry::periodic_table::atomic_weight;
use crate::error::DomainResult;
use crate::units::constants::UNIVERSAL_GAS_CONSTANT;
use crate::units::MolarMass;

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
    let total_percentage: f64 = composition.iter().map(|(_, p)| p).sum();

    if total_percentage <= 0.0 {
        return Ok(MolarMass::new(0.0));
    }

    let mut mean_kg_per_mol = 0.0;

    for (formula, percentage) in composition {
        let mass = molar_mass_of(formula)?;
        let fraction = percentage / total_percentage;
        mean_kg_per_mol += mass.value() * fraction;
    }

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
            "CO2" | "SO2" | "N2O" | "NO2" => 4.5 * UNIVERSAL_GAS_CONSTANT,
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
    let total_percentage: f64 = composition.iter().map(|(_, p)| p).sum();

    if total_percentage <= 0.0 {
        return Ok(0.0);
    }

    let mut mean_molar_cp = 0.0;
    let mut mean_molar_mass_val = 0.0;

    for (formula, percentage) in composition {
        let molar_cp = molar_heat_capacity_of(formula)?;
        let molar_mass = molar_mass_of(formula)?.value();
        let fraction = percentage / total_percentage;

        mean_molar_cp += molar_cp * fraction;
        mean_molar_mass_val += molar_mass * fraction;
    }

    if mean_molar_mass_val <= 0.0 {
        return Ok(0.0);
    }

    Ok(mean_molar_cp / mean_molar_mass_val)
}
