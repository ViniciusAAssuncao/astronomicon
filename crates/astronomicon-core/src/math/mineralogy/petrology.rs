use crate::chemistry::abundance::{element_mass_fraction, ElementalAbundance};
use crate::chemistry::periodic_table::atomic_weight;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormativeMineralogy {
    pub quartz: f64,
    pub plagioclase: f64,
    pub k_feldspar: f64,
    pub pyroxene: f64,
    pub olivine: f64,
}

impl NormativeMineralogy {
    pub fn new(
        quartz: f64,
        plagioclase: f64,
        k_feldspar: f64,
        pyroxene: f64,
        olivine: f64,
    ) -> Self {
        let total = quartz + plagioclase + k_feldspar + pyroxene + olivine;
        if total > 0.0 {
            Self {
                quartz: quartz / total,
                plagioclase: plagioclase / total,
                k_feldspar: k_feldspar / total,
                pyroxene: pyroxene / total,
                olivine: olivine / total,
            }
        } else {
            Self {
                quartz: 0.0,
                plagioclase: 0.0,
                k_feldspar: 0.0,
                pyroxene: 0.0,
                olivine: 0.0,
            }
        }
    }

    pub fn is_felsic(&self) -> bool {
        self.quartz + self.k_feldspar + self.plagioclase > 0.65 && self.quartz > 0.10
    }

    pub fn is_mafic(&self) -> bool {
        self.pyroxene + self.olivine > 0.40
    }

    pub fn is_ultramafic(&self) -> bool {
        self.pyroxene + self.olivine > 0.70
    }

    pub fn felsic_fraction(&self) -> f64 {
        self.quartz + self.plagioclase + self.k_feldspar
    }

    pub fn mafic_fraction(&self) -> f64 {
        self.pyroxene + self.olivine
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OxideAbundance {
    pub formula: String,
    pub mass_fraction: f64,
}

impl OxideAbundance {
    pub fn new(formula: impl Into<String>, mass_fraction: f64) -> Self {
        Self {
            formula: formula.into(),
            mass_fraction,
        }
    }
}

pub fn normative_cipw_mineralogy(abundances: &[ElementalAbundance]) -> NormativeMineralogy {
    let get_moles = |sym: &str| -> f64 {
        let w = element_mass_fraction(abundances, sym);
        if let Some(aw) = atomic_weight(sym) {
            if aw > 0.0 && w > 0.0 { w / aw } else { 0.0 }
        } else {
            0.0
        }
    };

    let n_si = get_moles("Si");
    let n_al = get_moles("Al");
    let n_fe = get_moles("Fe");
    let n_mg = get_moles("Mg");
    let n_ca = get_moles("Ca");
    let n_na = get_moles("Na");
    let n_k = get_moles("K");

    let n_k2o = n_k * 0.5;
    let n_na2o = n_na * 0.5;
    let mut n_al2o3 = n_al * 0.5;
    let mut n_cao = n_ca;
    let mut n_fm = n_fe + n_mg;
    let mut n_sio2 = n_si;

    let x_mg = if n_fm > 0.0 { n_mg / n_fm } else { 0.5 };
    let m_fm = 24.305 * x_mg + 55.845 * (1.0 - x_mg);

    let alloc_k = n_k2o.min(n_al2o3);
    let n_or = 2.0 * alloc_k;
    n_al2o3 -= alloc_k;
    n_sio2 -= 3.0 * n_or;
    let mass_or = n_or * 278.33;

    let alloc_na = n_na2o.min(n_al2o3);
    let n_ab = 2.0 * alloc_na;
    n_al2o3 -= alloc_na;
    n_sio2 -= 3.0 * n_ab;
    let mass_ab = n_ab * 262.22;

    let alloc_ca = n_cao.min(n_al2o3);
    let n_an = alloc_ca;
    n_cao -= alloc_ca;
    n_sio2 -= 2.0 * n_an;
    let mass_an = n_an * 278.21;

    let mass_plagioclase = mass_ab + mass_an;
    let mass_k_feldspar = mass_or;

    let alloc_di = n_cao.min(n_fm);
    let n_di = alloc_di;
    n_fm -= alloc_di;
    n_sio2 -= 2.0 * n_di;
    let mass_di = n_di * (40.078 + m_fm + 120.16);

    let (mass_q, mass_hy, mass_ol) = if n_sio2 >= n_fm {
        let n_hy = n_fm;
        let n_q = (n_sio2 - n_hy).max(0.0);
        let m_hy = n_hy * (m_fm + 60.08);
        let m_q = n_q * 60.08;
        (m_q, m_hy, 0.0)
    } else if n_sio2 >= 0.5 * n_fm {
        let n_hy = (2.0 * n_sio2 - n_fm).max(0.0);
        let n_ol = (n_fm - n_sio2).max(0.0);
        let m_hy = n_hy * (m_fm + 60.08);
        let m_ol = n_ol * (2.0 * m_fm + 60.08);
        (0.0, m_hy, m_ol)
    } else {
        let n_ol = n_sio2.max(0.0);
        let m_ol = n_ol * (2.0 * m_fm + 60.08);
        (0.0, 0.0, m_ol)
    };

    let mass_pyroxene = mass_di + mass_hy;
    NormativeMineralogy::new(
        mass_q,
        mass_plagioclase,
        mass_k_feldspar,
        mass_pyroxene,
        mass_ol,
    )
}

pub fn calculate_dominant_oxides(abundances: &[ElementalAbundance]) -> Vec<OxideAbundance> {
    let mut oxides = Vec::new();
    let mut total_oxide_mass = 0.0;

    let oxide_factors = [
        ("SiO2", "Si", 2.1393),
        ("TiO2", "Ti", 1.6681),
        ("Al2O3", "Al", 1.8894),
        ("FeO", "Fe", 1.2865),
        ("MnO", "Mn", 1.2912),
        ("MgO", "Mg", 1.6582),
        ("CaO", "Ca", 1.3992),
        ("Na2O", "Na", 1.3479),
        ("K2O", "K", 1.2046),
        ("P2O5", "P", 2.2914),
    ];

    for (oxide_name, element_sym, factor) in oxide_factors {
        let w_elem = element_mass_fraction(abundances, element_sym);
        let mass = w_elem * factor;
        if mass > 0.0 {
            oxides.push((oxide_name.to_string(), mass));
            total_oxide_mass += mass;
        }
    }

    if total_oxide_mass <= 0.0 {
        return Vec::new();
    }

    let mut result: Vec<OxideAbundance> = oxides
        .into_iter()
        .map(|(name, mass)| OxideAbundance::new(name, mass / total_oxide_mass))
        .collect();

    result.sort_by(|a, b| {
        b.mass_fraction
            .partial_cmp(&a.mass_fraction)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}
