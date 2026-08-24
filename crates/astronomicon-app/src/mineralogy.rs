use crate::climate::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::geology::resolve_planetary_geology;
use crate::geophysics::resolve_planetary_core;
use crate::hierarchy::find_parent_star;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use astronomicon_core::chemistry::{
    c_o_molar_ratio, element_mass_fraction, fe_si_molar_ratio, mg_number, mg_si_molar_ratio,
    refractory_mass_fraction, solar_abundance_to_mass_fractions, volatile_mass_fraction,
    ElementalAbundance,
};
use astronomicon_core::domain::{Planet, TectonicRegime};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::mineralogy::{
    banded_iron_formation_potential, calculate_dominant_oxides, crustal_elemental_abundances,
    crustal_petrology, differentiate_core_mantle, disk_temperature_at_orbit,
    evaporite_deposit_potential, hydrothermal_vein_potential, magmatic_sulfide_potential,
    normative_cipw_mineralogy, pegmatite_ree_potential,
    planetary_bulk_composition_from_disk_temp, NormativeMineralogy, OxideAbundance,
};
use astronomicon_core::math::radiometry::stellar_luminosity;
use astronomicon_core::math::thermodynamics::MatterState;
use astronomicon_core::units::constants::{ASTRONOMICAL_UNIT, SOLAR_LUMINOSITY, SOLAR_RADIUS};
use astronomicon_core::units::{Duration, Length, Luminosity, Temperature};
use astronomicon_db::repositories::{atmosphere_repository, hydrosphere_repository, planet_repository};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryBulkCompositionDiagnostic {
    pub disk_temperature: Temperature,
    pub abundances: Vec<ElementalAbundance>,
    pub refractory_fraction: f64,
    pub volatile_fraction: f64,
    pub mg_si_ratio: f64,
    pub fe_si_ratio: f64,
    pub c_o_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryDifferentiationDiagnostic {
    pub core_mass_fraction: f64,
    pub mantle_mass_fraction: f64,
    pub bulk_composition: Vec<ElementalAbundance>,
    pub core_composition: Vec<ElementalAbundance>,
    pub mantle_composition: Vec<ElementalAbundance>,
    pub core_fe_fraction: f64,
    pub core_ni_fraction: f64,
    pub mantle_mg_si_ratio: f64,
    pub mantle_fe_si_ratio: f64,
    pub mantle_mg_number: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrustalPetrologyDiagnostic {
    pub tectonic_regime: TectonicRegime,
    pub has_water: bool,
    pub normative_mineralogy: NormativeMineralogy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryAbundanceDiagnostic {
    pub disk_temperature: Temperature,
    pub bulk_abundances: Vec<ElementalAbundance>,
    pub refractory_fraction: f64,
    pub volatile_fraction: f64,
    pub mg_si_ratio: f64,
    pub fe_si_ratio: f64,
    pub c_o_ratio: f64,
    pub core_mass_fraction: f64,
    pub mantle_mass_fraction: f64,
    pub core_abundances: Vec<ElementalAbundance>,
    pub mantle_abundances: Vec<ElementalAbundance>,
    pub crustal_abundances: Vec<ElementalAbundance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrustalMineralogyDiagnostic {
    pub tectonic_regime: TectonicRegime,
    pub has_water: bool,
    pub normative_mineralogy: NormativeMineralogy,
    pub dominant_oxides: Vec<OxideAbundance>,
    pub felsic_fraction: f64,
    pub mafic_fraction: f64,
}

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryMineralogyDiagnostic {
    pub abundance: PlanetaryAbundanceDiagnostic,
    pub crustal_mineralogy: CrustalMineralogyDiagnostic,
    pub ore_potential: OrePotentialDiagnostic,
}

pub async fn resolve_planetary_bulk_composition(
    pool: &SqlitePool,
    planet_id: Uuid,
) -> AppResult<PlanetaryBulkCompositionDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, planet.orbital_parent()).await?;

    let feh = star.metallicity().unwrap_or(0.0);
    let star_temp = star
        .effective_temperature()
        .unwrap_or_else(|| Temperature::new(5778.0));
    let star_radius = star.radius().unwrap_or_else(|| Length::new(SOLAR_RADIUS));

    let star_lum = if star_radius.value() > 0.0 && star_temp.value() > 0.0 {
        stellar_luminosity(star_radius, star_temp)
    } else {
        Luminosity::new(SOLAR_LUMINOSITY)
    };

    let semi_major_axis = planet
        .orbital_elements()
        .map(|e| e.semi_major_axis())
        .unwrap_or_else(|| Length::new(ASTRONOMICAL_UNIT));

    let disk_temp = disk_temperature_at_orbit(star_lum, semi_major_axis);
    let star_abundances = solar_abundance_to_mass_fractions(feh);
    let planet_abundances = planetary_bulk_composition_from_disk_temp(
        &star_abundances,
        disk_temp,
        planet.kind(),
    );

    let refractory = refractory_mass_fraction(&planet_abundances);
    let volatile = volatile_mass_fraction(&planet_abundances);
    let mg_si = mg_si_molar_ratio(&planet_abundances);
    let fe_si = fe_si_molar_ratio(&planet_abundances);
    let c_o = c_o_molar_ratio(&planet_abundances);

    Ok(PlanetaryBulkCompositionDiagnostic {
        disk_temperature: disk_temp,
        abundances: planet_abundances,
        refractory_fraction: refractory,
        volatile_fraction: volatile,
        mg_si_ratio: mg_si,
        fe_si_ratio: fe_si,
        c_o_ratio: c_o,
    })
}

pub async fn resolve_planetary_differentiation(
    pool: &SqlitePool,
    planet_id: Uuid,
) -> AppResult<PlanetaryDifferentiationDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let bulk_diag = resolve_planetary_bulk_composition(pool, planet_id).await?;
    let core_mass_fraction = planet.core_mass_fraction().unwrap_or(0.0);
    let mantle_mass_fraction = (1.0 - core_mass_fraction).max(0.0);

    let (core_comp, mantle_comp) =
        differentiate_core_mantle(&bulk_diag.abundances, core_mass_fraction);

    let core_fe = element_mass_fraction(&core_comp, "Fe");
    let core_ni = element_mass_fraction(&core_comp, "Ni");
    let mantle_mg_si = mg_si_molar_ratio(&mantle_comp);
    let mantle_fe_si = fe_si_molar_ratio(&mantle_comp);
    let mantle_mg_number = mg_number(&mantle_comp);

    Ok(PlanetaryDifferentiationDiagnostic {
        core_mass_fraction,
        mantle_mass_fraction,
        bulk_composition: bulk_diag.abundances,
        core_composition: core_comp,
        mantle_composition: mantle_comp,
        core_fe_fraction: core_fe,
        core_ni_fraction: core_ni,
        mantle_mg_si_ratio: mantle_mg_si,
        mantle_fe_si_ratio: mantle_fe_si,
        mantle_mg_number,
    })
}

pub async fn resolve_crustal_petrology(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<CrustalPetrologyDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let diff_diag = resolve_planetary_differentiation(pool, planet_id).await?;
    let geology_diag = resolve_planetary_geology(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag = resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;

    let has_water = hydro_diag
        .map(|h| h.liquid_depth.value() > 0.0 || h.ice_thickness.value() > 0.0)
        .unwrap_or(false)
        || planet.mantle_hydration_fraction().unwrap_or(0.0) > 0.001;

    let mineralogy = crustal_petrology(
        &diff_diag.mantle_composition,
        geology_diag.tectonic_regime,
        has_water,
    );

    Ok(CrustalPetrologyDiagnostic {
        tectonic_regime: geology_diag.tectonic_regime,
        has_water,
        normative_mineralogy: mineralogy,
    })
}

pub async fn resolve_planetary_mineralogy(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<PlanetaryMineralogyDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, planet.orbital_parent()).await?;

    let feh = star.metallicity().unwrap_or(0.0);
    let star_temp = star
        .effective_temperature()
        .unwrap_or_else(|| Temperature::new(5778.0));
    let star_radius = star.radius().unwrap_or_else(|| Length::new(SOLAR_RADIUS));

    let star_lum = if star_radius.value() > 0.0 && star_temp.value() > 0.0 {
        stellar_luminosity(star_radius, star_temp)
    } else {
        Luminosity::new(SOLAR_LUMINOSITY)
    };

    let semi_major_axis = planet
        .orbital_elements()
        .map(|e| e.semi_major_axis())
        .unwrap_or_else(|| Length::new(ASTRONOMICAL_UNIT));

    let disk_temp = disk_temperature_at_orbit(star_lum, semi_major_axis);
    let star_abundances = solar_abundance_to_mass_fractions(feh);
    let bulk_abundances = planetary_bulk_composition_from_disk_temp(
        &star_abundances,
        disk_temp,
        planet.kind(),
    );

    let refractory = refractory_mass_fraction(&bulk_abundances);
    let volatile = volatile_mass_fraction(&bulk_abundances);
    let mg_si = mg_si_molar_ratio(&bulk_abundances);
    let fe_si = fe_si_molar_ratio(&bulk_abundances);
    let c_o = c_o_molar_ratio(&bulk_abundances);

    let core_mass_fraction = planet.core_mass_fraction().unwrap_or(0.0);
    let mantle_mass_fraction = (1.0 - core_mass_fraction).max(0.0);

    let (core_comp, mantle_comp) = differentiate_core_mantle(&bulk_abundances, core_mass_fraction);

    let total_epoch = universe_epoch + at_epoch;
    let geology_diag = resolve_planetary_geology(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag = resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;
    let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await?;
    let atm_opt = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let surf_temp = resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;

    let has_water = hydro_diag
        .map(|h| h.liquid_depth.value() > 0.0 || h.ice_thickness.value() > 0.0)
        .unwrap_or(false)
        || planet.mantle_hydration_fraction().unwrap_or(0.0) > 0.001;

    let is_liquid_or_supercritical = hydro_diag
        .map(|h| h.dominant_state == MatterState::Liquid || h.dominant_state == MatterState::Supercritical)
        .unwrap_or(false);

    let is_liquid_ocean = hydro_diag
        .map(|h| h.liquid_depth.value() > 0.0 && !h.is_completely_frozen)
        .unwrap_or(false);

    let crustal_abundances = crustal_elemental_abundances(
        &mantle_comp,
        geology_diag.tectonic_regime,
        has_water,
        total_epoch,
        core_diag.convective_heat_flux,
    );

    let normative_min = normative_cipw_mineralogy(&crustal_abundances);
    let dominant_ox = calculate_dominant_oxides(&crustal_abundances);
    let felsic_frac = normative_min.felsic_fraction();
    let mafic_frac = normative_min.mafic_fraction();

    let salinity = hydro_opt
        .as_ref()
        .map(|h| h.salinity_or_solute_mass_fraction())
        .unwrap_or(0.0);
    let ocean_cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);
    let boiling_pt = if let Some(h) = hydro_opt.as_ref() {
        let press = atm_opt.as_ref().map(|a| a.surface_pressure()).unwrap_or(astronomicon_core::units::Pressure::new(0.0));
        h.boiling_point(press).unwrap_or(Temperature::new(373.15))
    } else {
        Temperature::new(373.15)
    };

    let has_oxidizing_gas = atm_opt
        .as_ref()
        .map(|atm| {
            atm.composition().iter().any(|c| {
                matches!(c.formula(), "O2" | "CO2" | "H2O" | "NO2" | "SO2") && c.percentage() > 0.05
            })
        })
        .unwrap_or(false);

    let mut deposits = Vec::new();

    let w_au = element_mass_fraction(&crustal_abundances, "Au");
    let (p_au, e_au) = hydrothermal_vein_potential(
        w_au,
        has_water,
        is_liquid_or_supercritical,
        core_diag.convective_heat_flux,
        geology_diag.tectonic_regime,
    );
    if p_au > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Hydrothermal Gold Vein".to_string(),
            target_element: "Au".to_string(),
            deposit_type: "HydrothermalVein".to_string(),
            probability: p_au,
            enrichment_factor: e_au,
            estimated_grade_ppm: w_au * 1.0e6 * e_au,
            description: "Hydrothermal fluid circulation driven by mantle convection depositing native gold".to_string(),
        });
    }

    let w_ag = element_mass_fraction(&crustal_abundances, "Ag");
    let (p_ag, e_ag) = hydrothermal_vein_potential(
        w_ag,
        has_water,
        is_liquid_or_supercritical,
        core_diag.convective_heat_flux,
        geology_diag.tectonic_regime,
    );
    if p_ag > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Hydrothermal Silver Vein".to_string(),
            target_element: "Ag".to_string(),
            deposit_type: "HydrothermalVein".to_string(),
            probability: p_ag,
            enrichment_factor: e_ag,
            estimated_grade_ppm: w_ag * 1.0e6 * e_ag,
            description: "Epithermal and mesothermal veins enriched in silver".to_string(),
        });
    }

    let w_cu = element_mass_fraction(&crustal_abundances, "Cu");
    let (p_cu, e_cu) = hydrothermal_vein_potential(
        w_cu,
        has_water,
        is_liquid_or_supercritical,
        core_diag.convective_heat_flux,
        geology_diag.tectonic_regime,
    );
    if p_cu > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Volcanogenic Massive Sulfide / Porphyry Copper".to_string(),
            target_element: "Cu".to_string(),
            deposit_type: "HydrothermalVMS".to_string(),
            probability: p_cu,
            enrichment_factor: e_cu,
            estimated_grade_ppm: w_cu * 1.0e6 * e_cu,
            description: "Submarine and magmatic hydrothermal copper concentration".to_string(),
        });
    }

    let w_li = element_mass_fraction(&crustal_abundances, "Li");
    let (p_li_evap, e_li_evap) = evaporite_deposit_potential(
        w_li,
        has_water,
        surf_temp,
        boiling_pt,
        salinity,
        ocean_cov,
    );
    if p_li_evap > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Lithium Salar / Evaporite Brine".to_string(),
            target_element: "Li".to_string(),
            deposit_type: "Evaporite".to_string(),
            probability: p_li_evap,
            enrichment_factor: e_li_evap,
            estimated_grade_ppm: w_li * 1.0e6 * e_li_evap,
            description: "Endorheic basin evaporation enriching dissolved lithium salts".to_string(),
        });
    }

    let w_na = element_mass_fraction(&crustal_abundances, "Na");
    let (p_evap, e_evap) = evaporite_deposit_potential(
        w_na,
        has_water,
        surf_temp,
        boiling_pt,
        salinity,
        ocean_cov,
    );
    if p_evap > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Halite and Potash Evaporite Beds".to_string(),
            target_element: "Na".to_string(),
            deposit_type: "Evaporite".to_string(),
            probability: p_evap,
            enrichment_factor: e_evap,
            estimated_grade_ppm: w_na * 1.0e6 * e_evap,
            description: "Massive evaporite salt formation from evaporated surface bodies".to_string(),
        });
    }

    let w_fe = element_mass_fraction(&crustal_abundances, "Fe");
    let (p_bif, e_bif) = banded_iron_formation_potential(
        w_fe,
        has_water,
        is_liquid_ocean,
        has_oxidizing_gas,
        total_epoch,
    );
    if p_bif > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Banded Iron Formation (BIF)".to_string(),
            target_element: "Fe".to_string(),
            deposit_type: "BandedIronFormation".to_string(),
            probability: p_bif,
            enrichment_factor: e_bif,
            estimated_grade_ppm: w_fe * 1.0e6 * e_bif,
            description: "Chemical precipitation of marine iron oxide bands upon oceanic oxidation".to_string(),
        });
    }

    let w_ni = element_mass_fraction(&crustal_abundances, "Ni");
    let (p_mag, e_mag) = magmatic_sulfide_potential(
        w_ni,
        w_cu,
        core_mass_fraction,
        core_diag.convective_heat_flux,
        geology_diag.tectonic_regime,
    );
    if p_mag > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Magmatic Nickel-Copper Sulfide".to_string(),
            target_element: "Ni".to_string(),
            deposit_type: "MagmaticSulfide".to_string(),
            probability: p_mag,
            enrichment_factor: e_mag,
            estimated_grade_ppm: w_ni * 1.0e6 * e_mag,
            description: "Sulfide immiscibility in mafic/ultramafic mantle-derived magma conduits".to_string(),
        });
    }

    let w_u = element_mass_fraction(&crustal_abundances, "U");
    let (p_peg, e_peg) = pegmatite_ree_potential(
        felsic_frac,
        geology_diag.tectonic_regime,
        total_epoch,
    );
    if p_peg > 0.01 {
        deposits.push(OreDepositEstimate {
            name: "Uranium-Thorium Pegmatite".to_string(),
            target_element: "U".to_string(),
            deposit_type: "Pegmatite".to_string(),
            probability: p_peg,
            enrichment_factor: e_peg,
            estimated_grade_ppm: w_u * 1.0e6 * e_peg,
            description: "Fractionated granitic melt and pegmatite vein incompatible element concentration".to_string(),
        });

        let w_ree = element_mass_fraction(&crustal_abundances, "La")
            + element_mass_fraction(&crustal_abundances, "Ce")
            + element_mass_fraction(&crustal_abundances, "Nd")
            + element_mass_fraction(&crustal_abundances, "Y");
        deposits.push(OreDepositEstimate {
            name: "Rare Earth Element (REE) Alkaline Intrusion".to_string(),
            target_element: "REE".to_string(),
            deposit_type: "Pegmatite".to_string(),
            probability: p_peg,
            enrichment_factor: e_peg,
            estimated_grade_ppm: w_ree * 1.0e6 * e_peg,
            description: "Late-stage magmatic fractionation concentrating rare earth elements".to_string(),
        });
    }

    deposits.sort_by(|a, b| b.probability.partial_cmp(&a.probability).unwrap_or(std::cmp::Ordering::Equal));

    let abundance_diag = PlanetaryAbundanceDiagnostic {
        disk_temperature: disk_temp,
        bulk_abundances,
        refractory_fraction: refractory,
        volatile_fraction: volatile,
        mg_si_ratio: mg_si,
        fe_si_ratio: fe_si,
        c_o_ratio: c_o,
        core_mass_fraction,
        mantle_mass_fraction,
        core_abundances: core_comp,
        mantle_abundances: mantle_comp,
        crustal_abundances,
    };

    let crustal_diag = CrustalMineralogyDiagnostic {
        tectonic_regime: geology_diag.tectonic_regime,
        has_water,
        normative_mineralogy: normative_min,
        dominant_oxides: dominant_ox,
        felsic_fraction: felsic_frac,
        mafic_fraction: mafic_frac,
    };

    let ore_diag = OrePotentialDiagnostic {
        deposits,
        hydrothermal_active: p_au > 0.15 || p_cu > 0.15,
        evaporite_active: p_li_evap > 0.15 || p_evap > 0.15,
        bif_active: p_bif > 0.15,
        gold_potential: p_au,
        iron_potential: p_bif.max(if w_fe > 0.05 { 0.4 } else { 0.1 }),
        uranium_potential: p_peg,
        lithium_potential: p_li_evap.max(p_peg * 0.5),
        copper_potential: p_cu.max(p_mag),
    };

    Ok(PlanetaryMineralogyDiagnostic {
        abundance: abundance_diag,
        crustal_mineralogy: crustal_diag,
        ore_potential: ore_diag,
    })
}
