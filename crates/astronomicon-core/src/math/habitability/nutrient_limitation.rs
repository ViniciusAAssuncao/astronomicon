use crate::chemistry::abundance::{element_mass_fraction, ElementalAbundance};
use crate::chemistry::bioessentiality::redfield_limitation_from_molar_fractions;
use crate::chemistry::periodic_table::atomic_weight;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FirstOrderNutrientLimitation {
    pub limiting_element: String,
    pub availability_factor: f64,
    pub c_ratio_to_redfield: f64,
    pub n_ratio_to_redfield: f64,
    pub p_ratio_to_redfield: f64,
    pub is_phosphorus_limited: bool,
    pub is_nitrogen_limited: bool,
    pub is_carbon_limited: bool,
}

impl FirstOrderNutrientLimitation {
    pub fn new(
        limiting_element: impl Into<String>,
        availability_factor: f64,
        c_ratio_to_redfield: f64,
        n_ratio_to_redfield: f64,
        p_ratio_to_redfield: f64,
    ) -> Self {
        let limiting = limiting_element.into();
        let is_p = limiting == "P";
        let is_n = limiting == "N";
        let is_c = limiting == "C";

        Self {
            limiting_element: limiting,
            availability_factor,
            c_ratio_to_redfield,
            n_ratio_to_redfield,
            p_ratio_to_redfield,
            is_phosphorus_limited: is_p,
            is_nitrogen_limited: is_n,
            is_carbon_limited: is_c,
        }
    }

    pub fn limiting_element(&self) -> &str {
        &self.limiting_element
    }

    pub fn availability_factor(&self) -> f64 {
        self.availability_factor
    }

    pub fn c_ratio_to_redfield(&self) -> f64 {
        self.c_ratio_to_redfield
    }

    pub fn n_ratio_to_redfield(&self) -> f64 {
        self.n_ratio_to_redfield
    }

    pub fn p_ratio_to_redfield(&self) -> f64 {
        self.p_ratio_to_redfield
    }

    pub fn is_phosphorus_limited(&self) -> bool {
        self.is_phosphorus_limited
    }

    pub fn is_nitrogen_limited(&self) -> bool {
        self.is_nitrogen_limited
    }

    pub fn is_carbon_limited(&self) -> bool {
        self.is_carbon_limited
    }
}

pub fn evaluate_first_order_crustal_nutrients(
    crustal_abundances: &[ElementalAbundance],
) -> FirstOrderNutrientLimitation {
    let w_c = element_mass_fraction(crustal_abundances, "C");
    let w_n = element_mass_fraction(crustal_abundances, "N");
    let w_p = element_mass_fraction(crustal_abundances, "P");

    let aw_c = atomic_weight("C").unwrap_or(12.011);
    let aw_n = atomic_weight("N").unwrap_or(14.007);
    let aw_p = atomic_weight("P").unwrap_or(30.974);

    let n_c = if aw_c > 0.0 && w_c > 0.0 { w_c / aw_c } else { 0.0 };
    let n_n = if aw_n > 0.0 && w_n > 0.0 { w_n / aw_n } else { 0.0 };
    let n_p = if aw_p > 0.0 && w_p > 0.0 { w_p / aw_p } else { 0.0 };

    let n_total = n_c + n_n + n_p;
    if n_total <= 0.0 {
        return FirstOrderNutrientLimitation::new("P", 0.0, 0.0, 0.0, 0.0);
    }

    let x_c = n_c / n_total;
    let x_n = n_n / n_total;
    let x_p = n_p / n_total;

    let redfield = redfield_limitation_from_molar_fractions(x_c, x_n, x_p);

    FirstOrderNutrientLimitation::new(
        redfield.limiting_element,
        redfield.availability_factor,
        redfield.c_ratio_to_redfield,
        redfield.n_ratio_to_redfield,
        redfield.p_ratio_to_redfield,
    )
}

pub fn evaluate_first_order_surface_nutrients(
    crustal_abundances: &[ElementalAbundance],
    atmospheric_n2_fraction: Option<f64>,
    atmospheric_co2_fraction: Option<f64>,
) -> FirstOrderNutrientLimitation {
    let w_c = element_mass_fraction(crustal_abundances, "C");
    let w_n = element_mass_fraction(crustal_abundances, "N");
    let w_p = element_mass_fraction(crustal_abundances, "P");

    let aw_c = atomic_weight("C").unwrap_or(12.011);
    let aw_n = atomic_weight("N").unwrap_or(14.007);
    let aw_p = atomic_weight("P").unwrap_or(30.974);

    let n2_boost = atmospheric_n2_fraction.unwrap_or(0.0).clamp(0.0, 1.0) * 0.05;
    let co2_boost = atmospheric_co2_fraction.unwrap_or(0.0).clamp(0.0, 1.0) * 0.02;

    let effective_w_c = w_c + co2_boost;
    let effective_w_n = w_n + n2_boost;
    let effective_w_p = w_p;

    let n_c = if aw_c > 0.0 && effective_w_c > 0.0 {
        effective_w_c / aw_c
    } else {
        0.0
    };
    let n_n = if aw_n > 0.0 && effective_w_n > 0.0 {
        effective_w_n / aw_n
    } else {
        0.0
    };
    let n_p = if aw_p > 0.0 && effective_w_p > 0.0 {
        effective_w_p / aw_p
    } else {
        0.0
    };

    let n_total = n_c + n_n + n_p;
    if n_total <= 0.0 {
        return FirstOrderNutrientLimitation::new("P", 0.0, 0.0, 0.0, 0.0);
    }

    let x_c = n_c / n_total;
    let x_n = n_n / n_total;
    let x_p = n_p / n_total;

    let redfield = redfield_limitation_from_molar_fractions(x_c, x_n, x_p);

    FirstOrderNutrientLimitation::new(
        redfield.limiting_element,
        redfield.availability_factor,
        redfield.c_ratio_to_redfield,
        redfield.n_ratio_to_redfield,
        redfield.p_ratio_to_redfield,
    )
}
