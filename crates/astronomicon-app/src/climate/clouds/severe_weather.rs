use crate::climate::circulation::{
    resolve_planetary_circulation, resolve_wind_profile_at_latitude,
};
use crate::climate::clouds::instability::resolve_convective_instability;
use crate::climate::clouds::tropopause::resolve_tropopause;
use crate::climate::condensable_species::resolve_condensable_species;
use crate::error::AppResult;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::clouds::{
    CloudMorphology, LightningPotential, StormMode, bulk_richardson_number,
    classify_storm_mode, evaluate_lightning_potential, is_cyclogenesis_favorable,
    tropical_cyclone_potential_intensity,
};
use astronomicon_core::math::rotation::coriolis_parameter;
use astronomicon_core::math::thermodynamics::{
    mixing_ratio_from_relative_humidity, saturation_mixing_ratio,
    saturation_vapor_pressure,
};
use astronomicon_core::units::constants::DEFAULT_MIXED_PHASE_DEPTH_METERS;
use astronomicon_core::units::{Angle, Duration, Length, Speed};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, hydrosphere_repository};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SevereWeatherDiagnostic {
    pub bulk_richardson_number: f64,
    pub storm_mode: StormMode,
    pub lightning_potential: LightningPotential,
    pub tropical_cyclone_potential_intensity: Speed,
    pub is_cyclogenesis_favorable: bool,
    pub bulk_wind_shear: Speed,
}

pub async fn resolve_severe_weather(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<SevereWeatherDiagnostic> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let (solvent_props, solvent_molar_mass, surface_humidity) =
        resolve_condensable_species(pool, planet_id).await?;
    let instability =
        resolve_convective_instability(pool, planet_id, universe_epoch, at_epoch).await?;
    let tropo = resolve_tropopause(pool, planet_id, universe_epoch, at_epoch).await?;
    let circulation =
        resolve_planetary_circulation(pool, planet_id, universe_epoch, at_epoch).await?;

    let lat_mid = Angle::new(45.0 * PI / 180.0);
    let wind_mid =
        resolve_wind_profile_at_latitude(pool, planet_id, lat_mid, universe_epoch, at_epoch)
            .await?;

    let bulk_shear = Speed::new(
        (wind_mid.jet_stream_speed.value() - wind_mid.surface_wind_speed.value()).abs(),
    );
    let brn = bulk_richardson_number(instability.cape, bulk_shear);
    let storm_mode = classify_storm_mode(brn, instability.cape);

    let mixed_phase_depth = Length::new(DEFAULT_MIXED_PHASE_DEPTH_METERS);
    let is_convective = instability.morphology == CloudMorphology::Convective;
    let lightning =
        evaluate_lightning_potential(instability.cape, mixed_phase_depth, is_convective);

    let surf_temp = tropo.surface_temperature;
    let outflow_temp = tropo.skin_temperature;
    let surf_press = atmosphere.surface_pressure();
    let atm_molar_mass = atmosphere.mean_molar_mass()?;

    let p_sat = saturation_vapor_pressure(surf_temp, &solvent_props);
    let r_sat = saturation_mixing_ratio(p_sat, surf_press, solvent_molar_mass, atm_molar_mass);
    let r_bl = mixing_ratio_from_relative_humidity(surface_humidity, r_sat);
    let ck_over_cd = 0.9;

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let ocean_cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);

    let v_pot =
        if ocean_cov <= 0.0 || surf_temp.value() >= solvent_props.critical_temperature.value() {
            Speed::new(0.0)
        } else {
            tropical_cyclone_potential_intensity(
                surf_temp,
                outflow_temp,
                solvent_props.critical_temperature,
                solvent_props.enthalpy_of_vaporization,
                solvent_molar_mass,
                r_sat,
                r_bl,
                ck_over_cd,
            )
        };

    let lat_tropical = Angle::new(15.0 * PI / 180.0);
    let f_coriolis = coriolis_parameter(circulation.angular_velocity, lat_tropical);
    let favorable = is_cyclogenesis_favorable(ocean_cov, v_pot, f_coriolis);

    Ok(SevereWeatherDiagnostic {
        bulk_richardson_number: brn,
        storm_mode,
        lightning_potential: lightning,
        tropical_cyclone_potential_intensity: v_pot,
        is_cyclogenesis_favorable: favorable,
        bulk_wind_shear: bulk_shear,
    })
}