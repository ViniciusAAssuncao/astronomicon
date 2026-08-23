use crate::chemistry::abundance::ElementalAbundance;
use crate::chemistry::geochemistry::{condensation_fraction, goldschmidt_class_of, GoldschmidtClass};
use crate::domain::PlanetKind;
use crate::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use crate::units::{Length, Luminosity, Temperature};
use std::f64::consts::PI;

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