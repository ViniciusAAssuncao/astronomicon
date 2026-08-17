use crate::chemistry::molecular_formula;
use crate::chemistry::periodic_table::atomic_weight;
use crate::error::DomainResult;
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
