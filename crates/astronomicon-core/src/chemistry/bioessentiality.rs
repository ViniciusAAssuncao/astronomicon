use crate::chemistry::abundance::{element_molar_fraction, ElementalAbundance};
use crate::chemistry::periodic_table::atomic_weight;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BioessentialClass {
    Chnops,
    TraceMetalCofactor,
    NonEssential,
}

impl BioessentialClass {
    pub fn is_chnops(&self) -> bool {
        matches!(self, Self::Chnops)
    }

    pub fn is_trace_metal(&self) -> bool {
        matches!(self, Self::TraceMetalCofactor)
    }

    pub fn is_essential(&self) -> bool {
        !matches!(self, Self::NonEssential)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementBioessentiality {
    symbol: String,
    class: BioessentialClass,
}

impl ElementBioessentiality {
    pub fn new(symbol: impl Into<String>, class: BioessentialClass) -> Self {
        Self {
            symbol: symbol.into(),
            class,
        }
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn class(&self) -> BioessentialClass {
        self.class
    }

    pub fn is_chnops(&self) -> bool {
        self.class.is_chnops()
    }

    pub fn is_trace_metal(&self) -> bool {
        self.class.is_trace_metal()
    }

    pub fn is_essential(&self) -> bool {
        self.class.is_essential()
    }
}

pub fn element_bioessentiality(symbol: &str) -> Option<ElementBioessentiality> {
    let class = match symbol {
        "C" | "H" | "N" | "O" | "P" | "S" => BioessentialClass::Chnops,
        "Fe" | "Mn" | "Zn" | "Cu" | "Mo" | "Co" | "Ni" | "Se" => {
            BioessentialClass::TraceMetalCofactor
        }
        sym if atomic_weight(sym).is_some() => BioessentialClass::NonEssential,
        _ => return None,
    };
    Some(ElementBioessentiality::new(symbol, class))
}

pub fn bioessential_class_of(symbol: &str) -> Option<BioessentialClass> {
    element_bioessentiality(symbol).map(|e| e.class())
}

pub fn is_bioessential(symbol: &str) -> bool {
    element_bioessentiality(symbol)
        .map(|e| e.is_essential())
        .unwrap_or(false)
}

pub const CHNOPS_ELEMENTS: &[&str] = &["C", "H", "N", "O", "P", "S"];
pub const TRACE_METAL_COFACTOR_ELEMENTS: &[&str] =
    &["Fe", "Mn", "Zn", "Cu", "Mo", "Co", "Ni", "Se"];

pub fn chnops_elements() -> &'static [&'static str] {
    CHNOPS_ELEMENTS
}

pub fn trace_metal_cofactor_elements() -> &'static [&'static str] {
    TRACE_METAL_COFACTOR_ELEMENTS
}

pub const REDFIELD_RATIO_C: f64 = 106.0;
pub const REDFIELD_RATIO_N: f64 = 16.0;
pub const REDFIELD_RATIO_P: f64 = 1.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RedfieldLimitation {
    pub limiting_element: String,
    pub availability_factor: f64,
    pub c_ratio_to_redfield: f64,
    pub n_ratio_to_redfield: f64,
    pub p_ratio_to_redfield: f64,
}

impl RedfieldLimitation {
    pub fn new(
        limiting_element: impl Into<String>,
        availability_factor: f64,
        c_ratio_to_redfield: f64,
        n_ratio_to_redfield: f64,
        p_ratio_to_redfield: f64,
    ) -> Self {
        Self {
            limiting_element: limiting_element.into(),
            availability_factor,
            c_ratio_to_redfield,
            n_ratio_to_redfield,
            p_ratio_to_redfield,
        }
    }
}

pub fn evaluate_redfield_limitation(abundances: &[ElementalAbundance]) -> RedfieldLimitation {
    let x_c = element_molar_fraction(abundances, "C");
    let x_n = element_molar_fraction(abundances, "N");
    let x_p = element_molar_fraction(abundances, "P");

    redfield_limitation_from_molar_fractions(x_c, x_n, x_p)
}

pub fn redfield_limitation_from_molar_fractions(
    c_molar_fraction: f64,
    n_molar_fraction: f64,
    p_molar_fraction: f64,
) -> RedfieldLimitation {
    let r_c = if c_molar_fraction > 0.0 && c_molar_fraction.is_finite() {
        c_molar_fraction / REDFIELD_RATIO_C
    } else {
        0.0
    };
    let r_n = if n_molar_fraction > 0.0 && n_molar_fraction.is_finite() {
        n_molar_fraction / REDFIELD_RATIO_N
    } else {
        0.0
    };
    let r_p = if p_molar_fraction > 0.0 && p_molar_fraction.is_finite() {
        p_molar_fraction / REDFIELD_RATIO_P
    } else {
        0.0
    };

    let r_max = r_c.max(r_n).max(r_p);
    let r_min = r_c.min(r_n).min(r_p);

    if r_max <= 0.0 || !r_max.is_finite() {
        return RedfieldLimitation::new("P", 0.0, 0.0, 0.0, 0.0);
    }

    let availability_factor = (r_min / r_max).clamp(0.0, 1.0);

    let limiting_element = if r_c <= r_n && r_c <= r_p {
        "C"
    } else if r_n <= r_p {
        "N"
    } else {
        "P"
    };

    let c_norm = (r_c / r_max).clamp(0.0, 1.0);
    let n_norm = (r_n / r_max).clamp(0.0, 1.0);
    let p_norm = (r_p / r_max).clamp(0.0, 1.0);

    RedfieldLimitation::new(
        limiting_element,
        availability_factor,
        c_norm,
        n_norm,
        p_norm,
    )
}

pub fn redfield_limiting_element(abundances: &[ElementalAbundance]) -> (String, f64) {
    let lim = evaluate_redfield_limitation(abundances);
    (lim.limiting_element, lim.availability_factor)
}
