use crate::chemistry::geochemistry::element_geochemistry;
use crate::chemistry::periodic_table::{all_elements, atomic_weight, solar_log_epsilon};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementalAbundance {
    pub symbol: String,
    pub mass_fraction: f64,
}

impl ElementalAbundance {
    pub fn new(symbol: impl Into<String>, mass_fraction: f64) -> Self {
        Self {
            symbol: symbol.into(),
            mass_fraction,
        }
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn mass_fraction(&self) -> f64 {
        self.mass_fraction
    }
}

pub fn solar_abundance_to_mass_fractions(feh: f64) -> Vec<ElementalAbundance> {
    let metallicity_scale = 10.0_f64.powf(feh);
    let elements = all_elements();
    let mut raw_masses = Vec::with_capacity(elements.len());
    let mut total_mass = 0.0;

    for &symbol in elements {
        let log_eps = match solar_log_epsilon(symbol) {
            Some(val) => val,
            None => continue,
        };
        let weight = match atomic_weight(symbol) {
            Some(w) => w,
            None => continue,
        };

        let number_abundance = if symbol == "H" || symbol == "He" {
            10.0_f64.powf(log_eps - 12.0)
        } else {
            10.0_f64.powf(log_eps - 12.0) * metallicity_scale
        };

        let mass = number_abundance * weight;
        raw_masses.push((symbol, mass));
        total_mass += mass;
    }

    if total_mass <= 0.0 {
        return Vec::new();
    }

    raw_masses
        .into_iter()
        .map(|(sym, mass)| ElementalAbundance::new(sym, mass / total_mass))
        .collect()
}

pub fn stellar_abundances(feh: f64) -> Vec<ElementalAbundance> {
    solar_abundance_to_mass_fractions(feh)
}

pub fn element_mass_fraction(abundances: &[ElementalAbundance], symbol: &str) -> f64 {
    abundances
        .iter()
        .find(|a| a.symbol() == symbol)
        .map(|a| a.mass_fraction())
        .unwrap_or(0.0)
}

pub fn element_molar_fraction(abundances: &[ElementalAbundance], symbol: &str) -> f64 {
    let _weight = match atomic_weight(symbol) {
        Some(w) if w > 0.0 => w,
        _ => return 0.0,
    };

    let mut total_moles = 0.0;
    let mut target_moles = 0.0;

    for a in abundances {
        if let Some(w) = atomic_weight(a.symbol()) {
            if w > 0.0 && a.mass_fraction() > 0.0 {
                let moles = a.mass_fraction() / w;
                total_moles += moles;
                if a.symbol() == symbol {
                    target_moles = moles;
                }
            }
        }
    }

    if total_moles <= 0.0 {
        0.0
    } else {
        target_moles / total_moles
    }
}

pub fn refractory_mass_fraction(abundances: &[ElementalAbundance]) -> f64 {
    let mut total = 0.0;
    for a in abundances {
        if let Some(geochem) = element_geochemistry(a.symbol()) {
            if geochem.is_refractory() {
                total += a.mass_fraction();
            }
        }
    }
    total
}

pub fn volatile_mass_fraction(abundances: &[ElementalAbundance]) -> f64 {
    let mut total = 0.0;
    for a in abundances {
        if let Some(geochem) = element_geochemistry(a.symbol()) {
            if geochem.is_highly_volatile() || geochem.is_moderately_volatile() {
                total += a.mass_fraction();
            }
        }
    }
    total
}

pub fn mg_si_molar_ratio(abundances: &[ElementalAbundance]) -> f64 {
    let w_mg = element_mass_fraction(abundances, "Mg");
    let w_si = element_mass_fraction(abundances, "Si");
    let a_mg = atomic_weight("Mg").unwrap_or(24.305);
    let a_si = atomic_weight("Si").unwrap_or(28.085);

    if w_si <= 0.0 {
        0.0
    } else {
        (w_mg / a_mg) / (w_si / a_si)
    }
}

pub fn fe_si_molar_ratio(abundances: &[ElementalAbundance]) -> f64 {
    let w_fe = element_mass_fraction(abundances, "Fe");
    let w_si = element_mass_fraction(abundances, "Si");
    let a_fe = atomic_weight("Fe").unwrap_or(55.845);
    let a_si = atomic_weight("Si").unwrap_or(28.085);

    if w_si <= 0.0 {
        0.0
    } else {
        (w_fe / a_fe) / (w_si / a_si)
    }
}

pub fn c_o_molar_ratio(abundances: &[ElementalAbundance]) -> f64 {
    let w_c = element_mass_fraction(abundances, "C");
    let w_o = element_mass_fraction(abundances, "O");
    let a_c = atomic_weight("C").unwrap_or(12.011);
    let a_o = atomic_weight("O").unwrap_or(15.999);

    if w_o <= 0.0 {
        0.0
    } else {
        (w_c / a_c) / (w_o / a_o)
    }
}
