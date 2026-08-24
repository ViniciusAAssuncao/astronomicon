use crate::chemistry::abundance::ElementalAbundance;
use crate::chemistry::geochemistry::{GoldschmidtClass, goldschmidt_class_of};
use crate::domain::TectonicRegime;
use crate::math::mineralogy::petrology::{NormativeMineralogy, normative_cipw_mineralogy};
use crate::units::{Duration, HeatFlux};

pub fn incompatible_partition_coefficient(symbol: &str) -> f64 {
    match symbol {
        "U" | "Th" => 0.001,
        "Cs" | "Rb" | "Ba" => 0.002,
        "La" | "Ce" | "Pr" | "Nd" | "Sm" | "Eu" | "Gd" | "Tb" | "Dy" | "Ho" | "Er" | "Tm"
        | "Yb" | "Lu" | "Y" => 0.005,
        "Nb" | "Ta" => 0.004,
        "Li" | "Be" | "B" => 0.015,
        "Zr" | "Hf" => 0.02,
        "Pb" => 0.01,
        "Au" | "Ag" => 0.008,
        "Cu" | "Zn" | "Mo" | "Sn" | "W" => 0.03,
        "K" => 0.05,
        "Na" => 0.08,
        "Al" => 0.10,
        "Ca" => 0.15,
        "Si" => 0.20,
        "Ti" => 0.05,
        "P" => 0.03,
        "Fe" => 0.40,
        "Mg" => 1.8,
        "Co" => 2.0,
        "Cr" => 2.5,
        "Ni" => 4.5,
        _ => match goldschmidt_class_of(symbol) {
            Some(GoldschmidtClass::Lithophile) => 0.10,
            Some(GoldschmidtClass::Siderophile) => 0.05,
            Some(GoldschmidtClass::Chalcophile) => 0.02,
            Some(GoldschmidtClass::Atmophile) | None => 0.01,
        },
    }
}

pub fn crustal_enrichment_factor(
    symbol: &str,
    age: Duration,
    convective_heat_flux: HeatFlux,
) -> f64 {
    let d = incompatible_partition_coefficient(symbol);
    let q = convective_heat_flux.value();
    let t = age.value();

    if q <= 0.0 || t <= 0.0 || !q.is_finite() || !t.is_finite() {
        return 1.0;
    }

    let q_ref = 0.040;
    let t_ref = 4.5e9 * 31557600.0;
    let xi = (q / q_ref).sqrt() * (t / t_ref).powf(0.75);
    let n = (xi * 0.25).clamp(0.0, 5.0);

    if d < 1.0 {
        let c_melt_ratio = 1.0 / (d + 0.10 * (1.0 - d));
        let factor = 1.0 + (c_melt_ratio - 1.0) * (1.0 - (-n).exp());
        factor.max(1.0)
    } else {
        let factor = 1.0 / (1.0 + (d - 1.0) * (1.0 - (-n).exp()));
        factor.clamp(0.01, 1.0)
    }
}

pub fn crustal_elemental_abundances(
    mantle_abundances: &[ElementalAbundance],
    regime: TectonicRegime,
    has_water: bool,
    age: Duration,
    convective_heat_flux: HeatFlux,
) -> Vec<ElementalAbundance> {
    let (f_si, f_al, f_fe, f_mg, f_ca, f_na, f_k) = match regime {
        TectonicRegime::PlateTectonics if has_water => (1.40, 1.50, 0.30, 0.20, 0.80, 2.00, 2.50),
        TectonicRegime::PlateTectonics => (1.15, 1.20, 0.70, 0.50, 1.00, 1.30, 1.50),
        TectonicRegime::StagnantLid | TectonicRegime::Inactive => {
            (1.05, 1.30, 0.85, 0.65, 1.20, 1.10, 1.10)
        }
        TectonicRegime::HeatPipe => (0.95, 0.90, 1.00, 1.00, 0.90, 0.80, 0.80),
        TectonicRegime::IceTectonics => (1.00, 1.00, 0.80, 0.80, 1.00, 1.00, 1.00),
    };

    let mut fractionated = Vec::with_capacity(mantle_abundances.len());
    let mut total_frac = 0.0;

    for item in mantle_abundances {
        let sym = item.symbol();
        let base_factor = match sym {
            "Si" => f_si,
            "Al" => f_al,
            "Fe" => f_fe,
            "Mg" => f_mg,
            "Ca" => f_ca,
            "Na" => f_na,
            "K" => f_k,
            _ => 1.0,
        };
        let enrichment = crustal_enrichment_factor(sym, age, convective_heat_flux);
        let m = item.mass_fraction() * base_factor * enrichment;
        fractionated.push((sym.to_string(), m));
        total_frac += m;
    }

    if total_frac > 0.0 {
        fractionated
            .into_iter()
            .map(|(s, m)| ElementalAbundance::new(s, m / total_frac))
            .collect()
    } else {
        mantle_abundances.to_vec()
    }
}

pub fn crustal_petrology(
    mantle_abundances: &[ElementalAbundance],
    regime: TectonicRegime,
    has_water: bool,
) -> NormativeMineralogy {
    let (f_si, f_al, f_fe, f_mg, f_ca, f_na, f_k) = match regime {
        TectonicRegime::PlateTectonics if has_water => (1.40, 1.50, 0.30, 0.20, 0.80, 2.00, 2.50),
        TectonicRegime::PlateTectonics => (1.15, 1.20, 0.70, 0.50, 1.00, 1.30, 1.50),
        TectonicRegime::StagnantLid | TectonicRegime::Inactive => {
            (1.05, 1.30, 0.85, 0.65, 1.20, 1.10, 1.10)
        }
        TectonicRegime::HeatPipe => (0.95, 0.90, 1.00, 1.00, 0.90, 0.80, 0.80),
        TectonicRegime::IceTectonics => (1.00, 1.00, 0.80, 0.80, 1.00, 1.00, 1.00),
    };

    let mut fractionated = Vec::with_capacity(mantle_abundances.len());
    let mut total_frac = 0.0;

    for item in mantle_abundances {
        let factor = match item.symbol() {
            "Si" => f_si,
            "Al" => f_al,
            "Fe" => f_fe,
            "Mg" => f_mg,
            "Ca" => f_ca,
            "Na" => f_na,
            "K" => f_k,
            _ => 1.0,
        };
        let m = item.mass_fraction() * factor;
        fractionated.push((item.symbol().to_string(), m));
        total_frac += m;
    }

    let crustal_abundances: Vec<ElementalAbundance> = if total_frac > 0.0 {
        fractionated
            .into_iter()
            .map(|(s, m)| ElementalAbundance::new(s, m / total_frac))
            .collect()
    } else {
        mantle_abundances.to_vec()
    };

    normative_cipw_mineralogy(&crustal_abundances)
}
