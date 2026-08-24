use crate::chemistry::abundance::ElementalAbundance;
use crate::chemistry::geochemistry::{GoldschmidtClass, goldschmidt_class_of};

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
