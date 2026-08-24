pub mod composition;
pub mod differentiation;
pub mod ore_deposits;
pub mod petrology;

pub use composition::*;
pub use differentiation::*;
pub use ore_deposits::*;
pub use petrology::*;

use crate::climate::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::geology::resolve_planetary_geology;
use crate::geophysics::resolve_planetary_core;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use astronomicon_core::chemistry::{ElementalAbundance, element_mass_fraction};
use astronomicon_core::domain::{Planet, TectonicRegime};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::mineralogy::{
    NormativeMineralogy, OxideAbundance, calculate_dominant_oxides, crustal_elemental_abundances,
    normative_cipw_mineralogy,
};
use astronomicon_core::math::thermodynamics::MatterState;
use astronomicon_core::units::{Duration, Pressure, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
pub struct PlanetaryMineralogyDiagnostic {
    pub abundance: PlanetaryAbundanceDiagnostic,
    pub crustal_mineralogy: CrustalMineralogyDiagnostic,
    pub ore_potential: OrePotentialDiagnostic,
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

    let bulk_diag = resolve_planetary_bulk_composition(pool, planet_id).await?;
    let diff_diag = resolve_planetary_differentiation(pool, planet_id).await?;

    let total_epoch = universe_epoch + at_epoch;
    let geology_diag = resolve_planetary_geology(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_diag =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;
    let core_diag = resolve_planetary_core(pool, planet_id, universe_epoch, at_epoch).await?;
    let atm_opt = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;

    let has_water = hydro_diag
        .map(|h| h.liquid_depth.value() > 0.0 || h.ice_thickness.value() > 0.0)
        .unwrap_or(false)
        || planet.mantle_hydration_fraction().unwrap_or(0.0) > 0.001;

    let is_liquid_or_supercritical = hydro_diag
        .map(|h| {
            h.dominant_state == MatterState::Liquid
                || h.dominant_state == MatterState::Supercritical
        })
        .unwrap_or(false);

    let is_liquid_ocean = hydro_diag
        .map(|h| h.liquid_depth.value() > 0.0 && !h.is_completely_frozen)
        .unwrap_or(false);

    let crustal_abundances = crustal_elemental_abundances(
        &diff_diag.mantle_composition,
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
        let press = atm_opt
            .as_ref()
            .map(|a| a.surface_pressure())
            .unwrap_or(Pressure::new(0.0));
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

    let (hydrothermal_deposits, p_au, p_cu) = resolve_hydrothermal_deposits(
        &crustal_abundances,
        has_water,
        is_liquid_or_supercritical,
        core_diag.convective_heat_flux,
        geology_diag.tectonic_regime,
    );

    let (evaporite_deposits, p_li_evap, p_evap) = resolve_evaporite_deposits(
        &crustal_abundances,
        has_water,
        surf_temp,
        boiling_pt,
        salinity,
        ocean_cov,
    );

    let (bif_deposits, p_bif) = resolve_banded_iron_deposits(
        &crustal_abundances,
        has_water,
        is_liquid_ocean,
        has_oxidizing_gas,
        total_epoch,
    );

    let (magmatic_deposits, p_mag) = resolve_magmatic_sulfide_deposits(
        &crustal_abundances,
        diff_diag.core_mass_fraction,
        core_diag.convective_heat_flux,
        geology_diag.tectonic_regime,
    );

    let (pegmatite_deposits, p_peg) = resolve_pegmatite_deposits(
        &crustal_abundances,
        felsic_frac,
        geology_diag.tectonic_regime,
        total_epoch,
    );

    let mut deposits = Vec::new();
    deposits.extend(hydrothermal_deposits);
    deposits.extend(evaporite_deposits);
    deposits.extend(bif_deposits);
    deposits.extend(magmatic_deposits);
    deposits.extend(pegmatite_deposits);

    deposits.sort_by(|a, b| {
        b.probability
            .partial_cmp(&a.probability)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let w_fe = element_mass_fraction(&crustal_abundances, "Fe");

    let abundance_diag = PlanetaryAbundanceDiagnostic {
        disk_temperature: bulk_diag.disk_temperature,
        bulk_abundances: bulk_diag.abundances,
        refractory_fraction: bulk_diag.refractory_fraction,
        volatile_fraction: bulk_diag.volatile_fraction,
        mg_si_ratio: bulk_diag.mg_si_ratio,
        fe_si_ratio: bulk_diag.fe_si_ratio,
        c_o_ratio: bulk_diag.c_o_ratio,
        core_mass_fraction: diff_diag.core_mass_fraction,
        mantle_mass_fraction: diff_diag.mantle_mass_fraction,
        core_abundances: diff_diag.core_composition,
        mantle_abundances: diff_diag.mantle_composition,
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
