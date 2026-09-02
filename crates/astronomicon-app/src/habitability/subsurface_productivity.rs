use crate::climate::condensable_species::resolve_condensable_species;
use crate::climate::temperature::resolve_global_mean_temperature;
use crate::error::AppResult;
use crate::hydrosphere::resolve_hydrosphere_diagnostics;
use crate::mineralogy::resolve_planetary_mineralogy;
use crate::volcanism::resolve_planetary_volcanism;
use astronomicon_core::chemistry::abundance::element_mass_fraction;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::habitability::{
    evaluate_subsurface_ocean_chemosynthesis, evaluate_surface_chemosynthesis,
    ChemosyntheticPathways,
};
use astronomicon_core::math::volcanism::VolcanicGasOutgassingRates;
use astronomicon_core::units::{Duration, Length, Luminosity, MassRate, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use uuid::Uuid;

pub async fn resolve_chemosynthetic_productivity(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<ChemosyntheticPathways> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let hydro_diag_opt =
        resolve_hydrosphere_diagnostics(pool, planet_id, universe_epoch, at_epoch).await?;
    let volc_diag_res =
        resolve_planetary_volcanism(pool, planet_id, universe_epoch, at_epoch).await;
    let mineralogy_res =
        resolve_planetary_mineralogy(pool, planet_id, universe_epoch, at_epoch).await;
    let surf_temp_res =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await;

    let volc_diag = match volc_diag_res {
        Ok(v) => v,
        Err(_) => {
            return Ok(ChemosyntheticPathways::new(
                Luminosity::new(0.0),
                Luminosity::new(0.0),
                Luminosity::new(0.0),
                Luminosity::new(0.0),
                Luminosity::new(0.0),
                Luminosity::new(0.0),
                0.05,
            ));
        }
    };

    if let Some(hydro_diag) = hydro_diag_opt {
        if hydro_diag.is_subsurface_ocean {
            let mineralogy = mineralogy_res?;
            let (solvent_props, _, _) = resolve_condensable_species(pool, planet_id).await?;
            let eq_radius = planet
                .equatorial_radius()
                .unwrap_or_else(|| Length::new(6371e3));
            let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
            let total_depth =
                Length::new(hydro_diag.ice_thickness.value() + hydro_diag.liquid_depth.value());
            let mantle_hydration = planet.mantle_hydration_fraction().unwrap_or(0.0);
            let c_o_ratio = mineralogy.abundance.c_o_ratio;
            let sulfur_frac =
                element_mass_fraction(&mineralogy.abundance.crustal_abundances, "S");
            let ocean_temp = hydro_diag.surface_freezing_point;

            return Ok(evaluate_subsurface_ocean_chemosynthesis(
                volc_diag.global_magma_production_rate,
                mantle_hydration,
                c_o_ratio,
                sulfur_frac,
                solvent_props.liquid_density,
                g,
                total_depth,
                ocean_temp,
                None,
            ));
        }
    }

    let surf_temp = surf_temp_res.unwrap_or_else(|_| Temperature::new(288.15));

    let outgassing_rates = VolcanicGasOutgassingRates {
        h2o: volc_diag.outgassing_rate_h2o,
        co2: volc_diag.outgassing_rate_co2,
        so2: MassRate::new(volc_diag.outgassing_rate_sulfur.value() * 0.85),
        h2s: MassRate::new(volc_diag.outgassing_rate_sulfur.value() * 0.15),
        total: MassRate::new(
            volc_diag.outgassing_rate_h2o.value()
                + volc_diag.outgassing_rate_co2.value()
                + volc_diag.outgassing_rate_sulfur.value(),
        ),
    };

    Ok(evaluate_surface_chemosynthesis(
        &outgassing_rates,
        None,
        surf_temp,
        None,
    ))
}
