use crate::climate::emission::resolve_star_emission_profile;
use crate::climate::precipitation::resolve_precipitation_diagnostic;
use crate::climate::temperature::{
    resolve_advective_surface_temperature, resolve_top_of_atmosphere_irradiance,
};
use crate::error::AppResult;
use crate::habitability::spatial_integration::standard_latitude_bands;
use crate::hierarchy::find_parent_star;
use crate::radiation::resolve_surface_radiation;
use crate::sky::optical_column::resolve_optical_column_at_latitude;
use astronomicon_core::chemistry::optics::{mean_gas_optical_properties, GasOpticalProperties};
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::habitability::{
    evaluate_annual_radiation_tolerance, evaluate_chemical_tolerance,
    ChemicalToleranceAssessment, ChemicalToleranceConfidence, RadiationToleranceAssessment,
};
use astronomicon_core::math::optics::{evaluate_uv_photobiology, UvPhotobiologyResult};
use astronomicon_core::math::precipitation::{CondensatePrimaryClass, PrecipitationPhase};
use astronomicon_core::math::thermodynamics::MatterState;
use astronomicon_core::units::{Angle, Duration, Length, Pressure, RadiationDose};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BiochemicalViabilityDiagnostic {
    pub radiation_tolerance: RadiationToleranceAssessment,
    pub chemical_tolerance: ChemicalToleranceAssessment,
    pub uv_photobiology: UvPhotobiologyResult,
    pub is_liquid_solvent: bool,
    pub composite_viability_score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GlobalBiochemicalViabilityDiagnostic {
    pub average_radiation_tolerance: RadiationToleranceAssessment,
    pub average_chemical_tolerance: ChemicalToleranceAssessment,
    pub average_uv_photobiology: UvPhotobiologyResult,
    pub liquid_solvent_surface_fraction: f64,
    pub global_viability_score: f64,
}

pub async fn resolve_biochemical_viability_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<BiochemicalViabilityDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let surf_rad = resolve_surface_radiation(pool, planet_id, universe_epoch, at_epoch).await?;
    let lat_val = latitude.value();
    let cos2 = lat_val.cos() * lat_val.cos();
    let sin2 = lat_val.sin() * lat_val.sin();
    let local_dose = RadiationDose::new(
        surf_rad.equatorial_surface_dose.value() * cos2
            + surf_rad.polar_surface_dose.value() * sin2,
    );
    let radiation_tolerance = evaluate_annual_radiation_tolerance(local_dose);

    let surf_temp = resolve_advective_surface_temperature(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;

    let precip_res = resolve_precipitation_diagnostic(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await;

    let (cond_class, ph, phase, reaches_surface) = match &precip_res {
        Ok(p) => (p.primary_class, p.ph, p.phase, p.reaches_surface),
        Err(_) => (
            CondensatePrimaryClass::AqueousMolecular,
            Some(7.0),
            PrecipitationPhase::Liquid,
            false,
        ),
    };

    let chemical_tolerance = evaluate_chemical_tolerance(cond_class, surf_temp, ph);

    let star = find_parent_star(pool, planet.orbital_parent()).await?;
    let (_, star_temp, _) =
        resolve_star_emission_profile(pool, &star, universe_epoch, at_epoch).await?;
    let toa_irr =
        resolve_top_of_atmosphere_irradiance(pool, &planet, &star, universe_epoch, at_epoch)
            .await?;

    let atm_row = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));

    let uv_photobiology = if let Some(atm) = atm_row {
        let atm_comp: Vec<(String, f64)> = atm
            .composition()
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        let gas_opt = mean_gas_optical_properties(&atm_comp)?;
        let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
        let scale_h = atm.scale_height(g, surf_temp)?;
        let opt_col = resolve_optical_column_at_latitude(
            pool,
            planet_id,
            latitude,
            universe_epoch,
            at_epoch,
        )
        .await?;
        let h_val = scale_h.value().max(1.0);
        let mie_coeff = opt_col.aerosol_g / h_val;
        let aero_scale_h = Length::new(0.2 * h_val);

        evaluate_uv_photobiology(
            toa_irr,
            star_temp,
            &gas_opt,
            atm.surface_pressure(),
            surf_temp,
            scale_h,
            aero_scale_h,
            mie_coeff,
            Some(Angle::new(lat_val.abs())),
        )
    } else {
        let dummy_gas = GasOpticalProperties::new(0.0, 1.0, Vec::new());
        evaluate_uv_photobiology(
            toa_irr,
            star_temp,
            &dummy_gas,
            Pressure::new(0.0),
            surf_temp,
            Length::new(0.0),
            Length::new(0.0),
            0.0,
            Some(Angle::new(lat_val.abs())),
        )
    };

    let hydro_row = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let is_liquid_solvent = if let Some(hydro) = hydro_row {
        let press = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
            Some(a) => a.surface_pressure(),
            None => Pressure::new(0.0),
        };
        match hydro.matter_state(surf_temp, press) {
            Ok(MatterState::Liquid) => true,
            _ => phase == PrecipitationPhase::Liquid && reaches_surface,
        }
    } else {
        phase == PrecipitationPhase::Liquid && reaches_surface
    };

    let composite_viability_score = (radiation_tolerance.complex_life_survival_fraction
        * chemical_tolerance.overall_viability
        * uv_photobiology.dna_shielding_efficiency.max(0.05))
    .clamp(0.0, 1.0);

    Ok(BiochemicalViabilityDiagnostic {
        radiation_tolerance,
        chemical_tolerance,
        uv_photobiology,
        is_liquid_solvent,
        composite_viability_score,
    })
}

pub async fn resolve_global_biochemical_viability(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    sample_bands: usize,
) -> AppResult<GlobalBiochemicalViabilityDiagnostic> {
    let bands = standard_latitude_bands(sample_bands);

    let mut w_annual_dose = 0.0;
    let mut w_complex_surv = 0.0;
    let mut w_extrem_surv = 0.0;

    let mut w_temp_viab = 0.0;
    let mut w_ph_viab = 0.0;
    let mut count_ph = 0.0;
    let mut w_chem_overall = 0.0;
    let mut confidence = ChemicalToleranceConfidence::HighKnownAqueousBiochemistry;

    let mut w_toa_uv = 0.0;
    let mut w_surf_uv = 0.0;
    let mut w_dna_shield = 0.0;
    let mut w_uvc = 0.0;
    let mut w_uvb = 0.0;
    let mut w_uva = 0.0;

    let mut liquid_solvent_surface_fraction = 0.0;
    let mut global_viability_score = 0.0;

    for band in bands {
        let diag = resolve_biochemical_viability_at_latitude(
            pool,
            planet_id,
            band.latitude,
            universe_epoch,
            at_epoch,
        )
        .await?;

        w_annual_dose += band.weight * diag.radiation_tolerance.annual_dose.value();
        w_complex_surv += band.weight * diag.radiation_tolerance.complex_life_survival_fraction;
        w_extrem_surv += band.weight * diag.radiation_tolerance.extremophile_survival_fraction;

        w_temp_viab += band.weight * diag.chemical_tolerance.temperature_viability;
        if let Some(ph_v) = diag.chemical_tolerance.ph_viability {
            w_ph_viab += band.weight * ph_v;
            count_ph += band.weight;
        }
        w_chem_overall += band.weight * diag.chemical_tolerance.overall_viability;
        confidence = diag.chemical_tolerance.confidence;

        w_toa_uv += band.weight * diag.uv_photobiology.toa_effective_uv_irradiance.value();
        w_surf_uv += band.weight * diag.uv_photobiology.surface_effective_uv_irradiance.value();
        w_dna_shield += band.weight * diag.uv_photobiology.dna_shielding_efficiency;
        w_uvc += band.weight * diag.uv_photobiology.uvc_transmittance;
        w_uvb += band.weight * diag.uv_photobiology.uvb_transmittance;
        w_uva += band.weight * diag.uv_photobiology.uva_transmittance;

        if diag.is_liquid_solvent {
            liquid_solvent_surface_fraction += band.weight;
        }

        global_viability_score += band.weight * diag.composite_viability_score;
    }

    let threshold = 1.0 / std::f64::consts::E;
    let average_radiation_tolerance = RadiationToleranceAssessment::new(
        RadiationDose::new(w_annual_dose),
        w_complex_surv,
        w_extrem_surv,
        w_complex_surv >= threshold,
        w_extrem_surv >= threshold,
    );

    let average_chemical_tolerance = ChemicalToleranceAssessment::new(
        w_temp_viab,
        if count_ph > 0.0 {
            Some(w_ph_viab / count_ph)
        } else {
            None
        },
        w_chem_overall,
        confidence,
    );

    let average_uv_photobiology = UvPhotobiologyResult {
        toa_effective_uv_irradiance: astronomicon_core::units::Irradiance::new(w_toa_uv),
        surface_effective_uv_irradiance: astronomicon_core::units::Irradiance::new(w_surf_uv),
        dna_shielding_efficiency: w_dna_shield,
        uvc_transmittance: w_uvc,
        uvb_transmittance: w_uvb,
        uva_transmittance: w_uva,
    };

    Ok(GlobalBiochemicalViabilityDiagnostic {
        average_radiation_tolerance,
        average_chemical_tolerance,
        average_uv_photobiology,
        liquid_solvent_surface_fraction: liquid_solvent_surface_fraction.clamp(0.0, 1.0),
        global_viability_score: global_viability_score.clamp(0.0, 1.0),
    })
}
