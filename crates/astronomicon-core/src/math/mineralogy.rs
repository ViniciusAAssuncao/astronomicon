use crate::chemistry::abundance::{element_mass_fraction, ElementalAbundance};
use crate::chemistry::geochemistry::{condensation_fraction, goldschmidt_class_of, GoldschmidtClass};
use crate::chemistry::periodic_table::atomic_weight;
use crate::domain::{PlanetKind, TectonicRegime};
use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{Length, Luminosity, Temperature};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

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
}

pub fn normative_cipw_mineralogy(abundances: &[ElementalAbundance]) -> NormativeMineralogy {
    let get_moles = |sym: &str| -> f64 {
        let w = element_mass_fraction(abundances, sym);
        if let Some(aw) = atomic_weight(sym) {
            if aw > 0.0 && w > 0.0 {
                w / aw
            } else {
                0.0
            }
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
    NormativeMineralogy::new(mass_q, mass_plagioclase, mass_k_feldspar, mass_pyroxene, mass_ol)
}

pub fn crustal_petrology(
    mantle_abundances: &[ElementalAbundance],
    regime: TectonicRegime,
    has_water: bool,
) -> NormativeMineralogy {
    let (f_si, f_al, f_fe, f_mg, f_ca, f_na, f_k) = match regime {
        TectonicRegime::PlateTectonics if has_water => (1.40, 1.50, 0.30, 0.20, 0.80, 2.00, 2.50),
        TectonicRegime::PlateTectonics => (1.15, 1.20, 0.70, 0.50, 1.00, 1.30, 1.50),
        TectonicRegime::StagnantLid | TectonicRegime::Inactive => (1.05, 1.30, 0.85, 0.65, 1.20, 1.10, 1.10),
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

pub fn protoplanetary_disk_temperature(
    star_luminosity: Luminosity,
    orbital_distance: Length,
    disk_albedo: f64,
) -> Temperature {
    let l = star_luminosity.value();
    let r = orbital_distance.value();
    let a = disk_albedo.clamp(0.0, 1.0);

    if l <= 0.0 || r <= 0.0 || !l.is_finite() || !r.is_finite() {
        return Temperature::new(0.0);
    }

    let num = l * (1.0 - a);
    let den = 16.0 * PI * STEFAN_BOLTZMANN_CONSTANT * r * r;

    if den <= 0.0 || !den.is_finite() {
        return Temperature::new(0.0);
    }

    let t4 = num / den;
    if !t4.is_finite() || t4 <= 0.0 {
        Temperature::new(0.0)
    } else {
        Temperature::new(t4.powf(0.25))
    }
}

pub fn disk_temperature_at_orbit(
    star_luminosity: Luminosity,
    orbital_distance: Length,
) -> Temperature {
    protoplanetary_disk_temperature(star_luminosity, orbital_distance, 0.5)
}

pub fn thermal_condensation_efficiency(
    condensation_temperature: Temperature,
    local_temperature: Temperature,
    transition_width: f64,
) -> f64 {
    let tc = condensation_temperature.value();
    let td = local_temperature.value();

    if !tc.is_finite() || !td.is_finite() || tc <= 0.0 {
        return 0.0;
    }

    if td <= 0.0 {
        return 1.0;
    }

    let width = if transition_width.is_finite() && transition_width > 0.0 {
        transition_width
    } else {
        50.0
    };

    let scale = width * 0.25;
    let arg = (td - tc) / scale;

    if arg >= 50.0 {
        0.0
    } else if arg <= -50.0 {
        1.0
    } else {
        let val = 1.0 / (1.0 + arg.exp());
        val.clamp(0.0, 1.0)
    }
}

pub fn condensation_fraction_with_tc(
    condensation_temp: Temperature,
    disk_temperature: Temperature,
    transition_width: f64,
) -> f64 {
    thermal_condensation_efficiency(condensation_temp, disk_temperature, transition_width)
}

pub fn planetary_bulk_composition_from_disk_temp(
    star_abundances: &[ElementalAbundance],
    disk_temperature: Temperature,
    planet_kind: PlanetKind,
) -> Vec<ElementalAbundance> {
    let transition_width = 50.0;
    let si_fraction = condensation_fraction("Si", disk_temperature, transition_width);

    let mut raw_fractions = Vec::with_capacity(star_abundances.len());
    let mut total_raw = 0.0;

    for item in star_abundances {
        let sym = item.symbol();
        let w_star = item.mass_fraction();

        if w_star <= 0.0 {
            raw_fractions.push((sym.to_string(), 0.0));
            continue;
        }

        let mut eta = condensation_fraction(sym, disk_temperature, transition_width);

        if sym == "O" {
            eta = eta.max(0.6 * si_fraction);
        }

        let raw_m = match planet_kind {
            PlanetKind::Telluric | PlanetKind::Chthonian => w_star * eta,
            PlanetKind::CarbonPlanet => {
                if sym == "C" {
                    eta = eta.max(si_fraction);
                }
                w_star * eta
            }
            PlanetKind::GasGiant => {
                if sym == "H" || sym == "He" {
                    w_star
                } else {
                    w_star * (0.05 + 0.95 * eta)
                }
            }
            PlanetKind::IceGiant => {
                if sym == "H" || sym == "He" {
                    w_star * 0.08
                } else {
                    w_star * eta
                }
            }
            PlanetKind::IcyBody | PlanetKind::DwarfPlanet => {
                if sym == "H" || sym == "He" {
                    0.0
                } else {
                    w_star * eta
                }
            }
            PlanetKind::Exotic => w_star * eta,
        };

        raw_fractions.push((sym.to_string(), raw_m));
        total_raw += raw_m;
    }

    if total_raw <= 0.0 {
        return star_abundances.to_vec();
    }

    raw_fractions
        .into_iter()
        .map(|(sym, m)| ElementalAbundance::new(sym, m / total_raw))
        .collect()
}

pub fn planetary_bulk_composition(
    star_abundances: &[ElementalAbundance],
    orbital_distance: Length,
    star_luminosity: Luminosity,
    planet_kind: PlanetKind,
) -> Vec<ElementalAbundance> {
    let disk_temp = disk_temperature_at_orbit(star_luminosity, orbital_distance);
    planetary_bulk_composition_from_disk_temp(star_abundances, disk_temp, planet_kind)
}

pub fn metal_silicate_partition_coefficient(symbol: &str) -> f64 {
    match symbol {
        "Pt" | "Ir" | "Os" => 50000.0,
        "Re" => 40000.0,
        "Ru" | "Rh" => 30000.0,
        "Au" | "Pd" => 10000.0,
        "Mo" => 200.0,
        "Ni" => 150.0,
        "Co" => 80.0,
        "W" => 50.0,
        "Fe" | "Ge" => 40.0,
        "P" => 30.0,
        "S" | "Se" | "Te" => 20.0,
        "Ag" => 15.0,
        "Cu" | "As" => 10.0,
        "C" | "Sb" => 8.0,
        "Sn" => 5.0,
        "Ga" => 3.0,
        "In" => 2.5,
        "N" | "Tl" | "Pb" | "Bi" => 2.0,
        "Cr" | "Zn" | "Cd" => 1.5,
        "Hg" => 1.2,
        "V" => 0.8,
        "H" => 0.5,
        "Mn" => 0.3,
        "Si" => 0.05,
        "O" => 0.02,
        "Cl" | "Br" | "I" => 0.005,
        "He" | "Ne" | "Ar" | "Kr" | "Xe" => 0.0001,
        other => match goldschmidt_class_of(other) {
            Some(GoldschmidtClass::Siderophile) => 100.0,
            Some(GoldschmidtClass::Chalcophile) => 5.0,
            Some(GoldschmidtClass::Atmophile) => 0.01,
            Some(GoldschmidtClass::Lithophile) | None => 0.0001,
        },
    }
}

pub fn differentiate_core_mantle(
    bulk_composition: &[ElementalAbundance],
    core_mass_fraction: f64,
) -> (Vec<ElementalAbundance>, Vec<ElementalAbundance>) {
    let cmf = core_mass_fraction.clamp(0.0, 1.0);

    if cmf <= 0.0 {
        let core = bulk_composition
            .iter()
            .map(|a| ElementalAbundance::new(a.symbol(), 0.0))
            .collect();
        let mantle = bulk_composition.to_vec();
        return (core, mantle);
    }

    if cmf >= 1.0 {
        let core = bulk_composition.to_vec();
        let mantle = bulk_composition
            .iter()
            .map(|a| ElementalAbundance::new(a.symbol(), 0.0))
            .collect();
        return (core, mantle);
    }

    let mmf = 1.0 - cmf;
    let mut core_raw = Vec::with_capacity(bulk_composition.len());
    let mut mantle_raw = Vec::with_capacity(bulk_composition.len());
    let mut core_total = 0.0;
    let mut mantle_total = 0.0;

    for item in bulk_composition {
        let sym = item.symbol();
        let w_bulk = item.mass_fraction();

        if w_bulk <= 0.0 {
            core_raw.push((sym.to_string(), 0.0));
            mantle_raw.push((sym.to_string(), 0.0));
            continue;
        }

        let d_i = metal_silicate_partition_coefficient(sym);
        let denom = mmf + cmf * d_i;

        let c_mantle = if denom > 0.0 { w_bulk / denom } else { 0.0 };
        let c_core = d_i * c_mantle;

        core_raw.push((sym.to_string(), c_core));
        mantle_raw.push((sym.to_string(), c_mantle));

        core_total += c_core;
        mantle_total += c_mantle;
    }

    let core_res = if core_total > 0.0 {
        core_raw
            .into_iter()
            .map(|(s, val)| ElementalAbundance::new(s, val / core_total))
            .collect()
    } else {
        core_raw
            .into_iter()
            .map(|(s, _)| ElementalAbundance::new(s, 0.0))
            .collect()
    };

    let mantle_res = if mantle_total > 0.0 {
        mantle_raw
            .into_iter()
            .map(|(s, val)| ElementalAbundance::new(s, val / mantle_total))
            .collect()
    } else {
        mantle_raw
            .into_iter()
            .map(|(s, _)| ElementalAbundance::new(s, 0.0))
            .collect()
    };

    (core_res, mantle_res)
}

pub fn bulk_silicate_planet_composition(
    bulk_composition: &[ElementalAbundance],
    core_mass_fraction: f64,
) -> Vec<ElementalAbundance> {
    let (_, mantle) = differentiate_core_mantle(bulk_composition, core_mass_fraction);
    mantle
}

pub fn core_composition(
    bulk_composition: &[ElementalAbundance],
    core_mass_fraction: f64,
) -> Vec<ElementalAbundance> {
    let (core, _) = differentiate_core_mantle(bulk_composition, core_mass_fraction);
    core
}