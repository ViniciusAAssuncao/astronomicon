use crate::climate::emission::resolve_star_emission_profile;
use crate::climate::precipitation::resolve_precipitation_diagnostic;
use crate::climate::temperature::{
    resolve_advective_surface_temperature,
    resolve_top_of_atmosphere_irradiance,
};
use crate::error::AppResult;
use crate::habitability::spatial_integration::standard_latitude_bands;
use crate::hierarchy::find_parent_star;
use crate::sky::optical_column::resolve_optical_column_at_latitude;
use astronomicon_core::chemistry::optics::{ mean_gas_optical_properties };
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{ gravitational_parameter, surface_gravity };
use astronomicon_core::math::habitability::{
    standard_primary_habitability_from_rate,
    PrimaryProductivityConfidence,
    StandardPrimaryHabitability,
};
use astronomicon_core::math::precipitation::CondensatePrimaryClass;
use astronomicon_core::math::radiometry::{
    evaluate_photosynthetic_flux,
    par_spectral_fraction,
    theoretical_max_biomass_energy_flux,
    top_of_atmosphere_par_irradiance,
    PhotosyntheticFluxSummary,
};
use astronomicon_core::units::{ Angle, Duration, Irradiance, Length, Speed };
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{ atmosphere_repository, planet_repository };
use serde::{ Deserialize, Serialize };
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurfaceProductivityDiagnostic {
    pub theoretical_photosynthetic_flux: PhotosyntheticFluxSummary,
    pub empirical_primary_habitability: StandardPrimaryHabitability,
}

pub async fn resolve_photosynthetic_productivity_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<PhotosyntheticFluxSummary> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, planet.orbital_parent()).await?;
    let (_, star_temp, _) = resolve_star_emission_profile(
        pool,
        &star,
        universe_epoch,
        at_epoch
    ).await?;
    let toa_irradiance = resolve_top_of_atmosphere_irradiance(
        pool,
        &planet,
        &star,
        universe_epoch,
        at_epoch
    ).await?;

    let surf_temp = resolve_advective_surface_temperature(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;

    let atm_row = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    if let Some(atm) = atm_row {
        let atm_comp: Vec<(String, f64)> = atm
            .composition()
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        let gas_opt = mean_gas_optical_properties(&atm_comp)?;

        let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
        let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
        let scale_h = atm.scale_height(g, surf_temp)?;
        let opt_col = resolve_optical_column_at_latitude(
            pool,
            planet_id,
            latitude,
            universe_epoch,
            at_epoch
        ).await?;

        let h_val = scale_h.value().max(1.0);
        let mie_coeff = opt_col.aerosol_g / h_val;
        let aero_scale_h = Length::new(0.2 * h_val);

        Ok(
            evaluate_photosynthetic_flux(
                toa_irradiance,
                star_temp,
                &gas_opt,
                atm.surface_pressure(),
                surf_temp,
                scale_h,
                aero_scale_h,
                mie_coeff,
                Some(Angle::new(latitude.value().abs()))
            )
        )
    } else {
        let par_frac = par_spectral_fraction(star_temp);
        let toa_par = top_of_atmosphere_par_irradiance(toa_irradiance, star_temp);
        let max_biomass = theoretical_max_biomass_energy_flux(toa_par);

        Ok(PhotosyntheticFluxSummary {
            toa_par_irradiance: toa_par,
            surface_par_irradiance: toa_par,
            par_fraction_of_total: par_frac,
            atmospheric_par_transmittance: 1.0,
            max_biomass_energy_flux: max_biomass,
        })
    }
}

pub async fn resolve_standard_primary_habitability_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<StandardPrimaryHabitability> {
    let surf_temp = resolve_advective_surface_temperature(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;

    let precip = resolve_precipitation_diagnostic(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await.ok();

    let (precip_rate, is_aqueous) = match precip {
        Some(p) =>
            (
                p.linear_accumulation_rate,
                p.primary_class == CondensatePrimaryClass::AqueousMolecular,
            ),
        None => (Speed::new(0.0), true),
    };

    Ok(standard_primary_habitability_from_rate(surf_temp, precip_rate, is_aqueous))
}

pub async fn resolve_surface_productivity_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<SurfaceProductivityDiagnostic> {
    let theoretical_photosynthetic_flux = resolve_photosynthetic_productivity_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;

    let empirical_primary_habitability = resolve_standard_primary_habitability_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;

    Ok(SurfaceProductivityDiagnostic {
        theoretical_photosynthetic_flux,
        empirical_primary_habitability,
    })
}

pub async fn resolve_global_surface_productivity(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    sample_bands: usize
) -> AppResult<SurfaceProductivityDiagnostic> {
    let bands = standard_latitude_bands(sample_bands);

    let mut w_toa_par = 0.0;
    let mut w_surf_par = 0.0;
    let mut w_par_frac = 0.0;
    let mut w_trans = 0.0;
    let mut w_biomass = 0.0;

    let mut w_npp_t = 0.0;
    let mut w_npp_p = 0.0;
    let mut w_npp_final = 0.0;
    let mut w_sph = 0.0;
    let mut any_speculative = false;

    for band in bands {
        let diag = resolve_surface_productivity_at_latitude(
            pool,
            planet_id,
            band.latitude,
            universe_epoch,
            at_epoch
        ).await?;

        w_toa_par += band.weight * diag.theoretical_photosynthetic_flux.toa_par_irradiance.value();
        w_surf_par +=
            band.weight * diag.theoretical_photosynthetic_flux.surface_par_irradiance.value();
        w_par_frac += band.weight * diag.theoretical_photosynthetic_flux.par_fraction_of_total;
        w_trans += band.weight * diag.theoretical_photosynthetic_flux.atmospheric_par_transmittance;
        w_biomass +=
            band.weight * diag.theoretical_photosynthetic_flux.max_biomass_energy_flux.value();

        w_npp_t += band.weight * diag.empirical_primary_habitability.npp_temperature;
        w_npp_p += band.weight * diag.empirical_primary_habitability.npp_precipitation;
        w_npp_final += band.weight * diag.empirical_primary_habitability.npp_final;
        w_sph += band.weight * diag.empirical_primary_habitability.sph_index;

        if
            diag.empirical_primary_habitability.confidence ==
            PrimaryProductivityConfidence::LowNonAqueousSpeculative
        {
            any_speculative = true;
        }
    }

    let confidence = if any_speculative {
        PrimaryProductivityConfidence::LowNonAqueousSpeculative
    } else {
        PrimaryProductivityConfidence::HighAqueousBiochemistry
    };

    Ok(SurfaceProductivityDiagnostic {
        theoretical_photosynthetic_flux: PhotosyntheticFluxSummary {
            toa_par_irradiance: Irradiance::new(w_toa_par),
            surface_par_irradiance: Irradiance::new(w_surf_par),
            par_fraction_of_total: w_par_frac,
            atmospheric_par_transmittance: w_trans,
            max_biomass_energy_flux: Irradiance::new(w_biomass),
        },
        empirical_primary_habitability: StandardPrimaryHabitability::new(
            w_npp_t,
            w_npp_p,
            w_npp_final,
            w_sph,
            confidence
        ),
    })
}
