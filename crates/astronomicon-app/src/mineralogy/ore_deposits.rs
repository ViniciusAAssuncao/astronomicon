use astronomicon_core::chemistry::{element_mass_fraction, ElementalAbundance};
use astronomicon_core::domain::TectonicRegime;
use astronomicon_core::math::mineralogy::{
    banded_iron_formation_potential, evaporite_deposit_potential, hydrothermal_vein_potential,
    magmatic_sulfide_potential, pegmatite_ree_potential,
};
use astronomicon_core::units::{Duration, HeatFlux, Temperature};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OreDepositEstimate {
    pub name: String,
    pub target_element: String,
    pub deposit_type: String,
    pub probability: f64,
    pub enrichment_factor: f64,
    pub estimated_grade_ppm: f64,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrePotentialDiagnostic {
    pub deposits: Vec<OreDepositEstimate>,
    pub hydrothermal_active: bool,
    pub evaporite_active: bool,
    pub bif_active: bool,
    pub gold_potential: f64,
    pub iron_potential: f64,
    pub uranium_potential: f64,
    pub lithium_potential: f64,
    pub copper_potential: f64,
}

fn push_deposit_if_significant(
    deposits: &mut Vec<OreDepositEstimate>,
    name: &str,
    target_element: &str,
    deposit_type: &str,
    probability: f64,
    enrichment_factor: f64,
    base_mass_fraction: f64,
    description: &str,
) {
    if probability > 0.01 {
        deposits.push(OreDepositEstimate {
            name: name.to_string(),
            target_element: target_element.to_string(),
            deposit_type: deposit_type.to_string(),
            probability,
            enrichment_factor,
            estimated_grade_ppm: base_mass_fraction * 1.0e6 * enrichment_factor,
            description: description.to_string(),
        });
    }
}

pub fn resolve_hydrothermal_deposits(
    crustal_abundances: &[ElementalAbundance],
    has_water: bool,
    is_liquid_or_supercritical: bool,
    convective_heat_flux: HeatFlux,
    tectonic_regime: TectonicRegime,
) -> (Vec<OreDepositEstimate>, f64, f64) {
    let mut deposits = Vec::new();

    let w_au = element_mass_fraction(crustal_abundances, "Au");
    let (p_au, e_au) = hydrothermal_vein_potential(
        w_au,
        has_water,
        is_liquid_or_supercritical,
        convective_heat_flux,
        tectonic_regime,
    );
    push_deposit_if_significant(
        &mut deposits,
        "Hydrothermal Gold Vein",
        "Au",
        "HydrothermalVein",
        p_au,
        e_au,
        w_au,
        "Hydrothermal fluid circulation driven by mantle convection depositing native gold",
    );

    let w_ag = element_mass_fraction(crustal_abundances, "Ag");
    let (p_ag, e_ag) = hydrothermal_vein_potential(
        w_ag,
        has_water,
        is_liquid_or_supercritical,
        convective_heat_flux,
        tectonic_regime,
    );
    push_deposit_if_significant(
        &mut deposits,
        "Hydrothermal Silver Vein",
        "Ag",
        "HydrothermalVein",
        p_ag,
        e_ag,
        w_ag,
        "Epithermal and mesothermal veins enriched in silver",
    );

    let w_cu = element_mass_fraction(crustal_abundances, "Cu");
    let (p_cu, e_cu) = hydrothermal_vein_potential(
        w_cu,
        has_water,
        is_liquid_or_supercritical,
        convective_heat_flux,
        tectonic_regime,
    );
    push_deposit_if_significant(
        &mut deposits,
        "Volcanogenic Massive Sulfide / Porphyry Copper",
        "Cu",
        "HydrothermalVMS",
        p_cu,
        e_cu,
        w_cu,
        "Submarine and magmatic hydrothermal copper concentration",
    );

    (deposits, p_au, p_cu)
}

pub fn resolve_evaporite_deposits(
    crustal_abundances: &[ElementalAbundance],
    has_water: bool,
    surface_temperature: Temperature,
    boiling_point: Temperature,
    salinity: f64,
    ocean_coverage: f64,
) -> (Vec<OreDepositEstimate>, f64, f64) {
    let mut deposits = Vec::new();

    let w_li = element_mass_fraction(crustal_abundances, "Li");
    let (p_li_evap, e_li_evap) = evaporite_deposit_potential(
        w_li,
        has_water,
        surface_temperature,
        boiling_point,
        salinity,
        ocean_coverage,
    );
    push_deposit_if_significant(
        &mut deposits,
        "Lithium Salar / Evaporite Brine",
        "Li",
        "Evaporite",
        p_li_evap,
        e_li_evap,
        w_li,
        "Endorheic basin evaporation enriching dissolved lithium salts",
    );

    let w_na = element_mass_fraction(crustal_abundances, "Na");
    let (p_evap, e_evap) = evaporite_deposit_potential(
        w_na,
        has_water,
        surface_temperature,
        boiling_point,
        salinity,
        ocean_coverage,
    );
    push_deposit_if_significant(
        &mut deposits,
        "Halite and Potash Evaporite Beds",
        "Na",
        "Evaporite",
        p_evap,
        e_evap,
        w_na,
        "Massive evaporite salt formation from evaporated surface bodies",
    );

    (deposits, p_li_evap, p_evap)
}

pub fn resolve_banded_iron_deposits(
    crustal_abundances: &[ElementalAbundance],
    has_water: bool,
    is_liquid_ocean: bool,
    has_oxidizing_gas: bool,
    total_epoch: Duration,
) -> (Vec<OreDepositEstimate>, f64) {
    let mut deposits = Vec::new();

    let w_fe = element_mass_fraction(crustal_abundances, "Fe");
    let (p_bif, e_bif) = banded_iron_formation_potential(
        w_fe,
        has_water,
        is_liquid_ocean,
        has_oxidizing_gas,
        total_epoch,
    );
    push_deposit_if_significant(
        &mut deposits,
        "Banded Iron Formation (BIF)",
        "Fe",
        "BandedIronFormation",
        p_bif,
        e_bif,
        w_fe,
        "Chemical precipitation of marine iron oxide bands upon oceanic oxidation",
    );

    (deposits, p_bif)
}

pub fn resolve_magmatic_sulfide_deposits(
    crustal_abundances: &[ElementalAbundance],
    core_mass_fraction: f64,
    convective_heat_flux: HeatFlux,
    tectonic_regime: TectonicRegime,
) -> (Vec<OreDepositEstimate>, f64) {
    let mut deposits = Vec::new();

    let w_ni = element_mass_fraction(crustal_abundances, "Ni");
    let w_cu = element_mass_fraction(crustal_abundances, "Cu");
    let (p_mag, e_mag) = magmatic_sulfide_potential(
        w_ni,
        w_cu,
        core_mass_fraction,
        convective_heat_flux,
        tectonic_regime,
    );
    push_deposit_if_significant(
        &mut deposits,
        "Magmatic Nickel-Copper Sulfide",
        "Ni",
        "MagmaticSulfide",
        p_mag,
        e_mag,
        w_ni,
        "Sulfide immiscibility in mafic/ultramafic mantle-derived magma conduits",
    );

    (deposits, p_mag)
}

pub fn resolve_pegmatite_deposits(
    crustal_abundances: &[ElementalAbundance],
    felsic_fraction: f64,
    tectonic_regime: TectonicRegime,
    total_epoch: Duration,
) -> (Vec<OreDepositEstimate>, f64) {
    let mut deposits = Vec::new();

    let (p_peg, e_peg) = pegmatite_ree_potential(
        felsic_fraction,
        tectonic_regime,
        total_epoch,
    );

    let w_u = element_mass_fraction(crustal_abundances, "U");
    push_deposit_if_significant(
        &mut deposits,
        "Uranium-Thorium Pegmatite",
        "U",
        "Pegmatite",
        p_peg,
        e_peg,
        w_u,
        "Fractionated granitic melt and pegmatite vein incompatible element concentration",
    );

    let w_ree = element_mass_fraction(crustal_abundances, "La")
        + element_mass_fraction(crustal_abundances, "Ce")
        + element_mass_fraction(crustal_abundances, "Nd")
        + element_mass_fraction(crustal_abundances, "Y");
    push_deposit_if_significant(
        &mut deposits,
        "Rare Earth Element (REE) Alkaline Intrusion",
        "REE",
        "Pegmatite",
        p_peg,
        e_peg,
        w_ree,
        "Late-stage magmatic fractionation concentrating rare earth elements",
    );

    (deposits, p_peg)
}