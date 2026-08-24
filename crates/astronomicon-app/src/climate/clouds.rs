use crate::climate::atmosphere::{
    resolve_atmospheric_stratification, resolve_atmospheric_stratification_at_latitude,
};
use crate::climate::circulation::{resolve_planetary_circulation, resolve_wind_profile_at_latitude};
use crate::climate::condensable_species::resolve_condensable_species;
use crate::climate::emission::resolve_star_emission_profile;
use crate::climate::temperature::resolve_advective_surface_temperature;
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use astronomicon_core::domain::{Planet, StarKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::black_hole::gravitational_redshift_between;
use astronomicon_core::math::climate::{
    local_radiative_equilibrium_temperature, temperature_at_altitude,
};
use astronomicon_core::math::clouds::{
    AtmosphericStability, CloudClassification, CloudMorphology, GlaciationState,
    LightningPotential, StormMode, adiabatic_condensate_density, bulk_richardson_number,
    classify_atmospheric_stability, classify_cloud_morphology, classify_cloud_system,
    classify_storm_mode, cloud_band_altitudes, combine_layer_cloud_fractions_max_random_overlap,
    evaluate_lightning_potential, freezing_level_altitude, glaciation_state_from_ice_fraction,
    ice_condensate_density, ice_fraction_at_altitude, is_cyclogenesis_favorable,
    is_deep_convective_cloud, layer_critical_relative_humidity, liquid_condensate_density,
    mixing_ratio_at_altitude, relative_humidity_at_altitude, sundqvist_cloud_fraction_with_ccn,
    tropical_cyclone_potential_intensity,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::radiometry::orbital_irradiance;
use astronomicon_core::math::rotation::coriolis_parameter;
use astronomicon_core::math::thermodynamics::{
    convective_available_potential_energy, convective_inhibition, dry_adiabatic_lapse_rate,
    equilibrium_level, grey_atmosphere_skin_temperature, integrate_parcel_buoyancy_profile,
    level_of_free_convection, mixing_ratio_from_relative_humidity, moist_adiabatic_lapse_rate,
    saturation_mixing_ratio, saturation_vapor_pressure, tropopause_altitude,
};
use astronomicon_core::units::constants::DEFAULT_MIXED_PHASE_DEPTH_METERS;
use astronomicon_core::units::{
    Angle, Density, Duration, Irradiance, Length, SpecificEnergy, Speed, Temperature,
    TemperatureGradient,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TropopauseDiagnostic {
    pub radiative_equilibrium_temperature: Temperature,
    pub skin_temperature: Temperature,
    pub surface_temperature: Temperature,
    pub tropopause_altitude: Length,
    pub tropopause_temperature: Temperature,
    pub lapse_rate: TemperatureGradient,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConvectiveInstabilityDiagnostic {
    pub surface_dew_point: Temperature,
    pub lcl_altitude: Length,
    pub lfc_altitude: Option<Length>,
    pub equilibrium_level: Option<Length>,
    pub cape: SpecificEnergy,
    pub cin: SpecificEnergy,
    pub stability: AtmosphericStability,
    pub morphology: CloudMorphology,
    pub is_deep_convection: bool,
    pub dry_adiabatic_lapse_rate: TemperatureGradient,
    pub moist_adiabatic_lapse_rate: TemperatureGradient,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudLayerDiagnostic {
    pub base_altitude: Length,
    pub top_altitude: Length,
    pub representative_altitude: Length,
    pub cloud_fraction: f64,
    pub relative_humidity: f64,
    pub critical_relative_humidity: f64,
    pub liquid_water_content: Density,
    pub ice_water_content: Density,
    pub ice_fraction: f64,
    pub glaciation_state: GlaciationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudCoverDiagnostic {
    pub total_cloud_fraction: f64,
    pub low_cloud: CloudLayerDiagnostic,
    pub mid_cloud: CloudLayerDiagnostic,
    pub high_cloud: CloudLayerDiagnostic,
    pub freezing_level: Length,
    pub classification: CloudClassification,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SevereWeatherDiagnostic {
    pub bulk_richardson_number: f64,
    pub storm_mode: StormMode,
    pub lightning_potential: LightningPotential,
    pub tropical_cyclone_potential_intensity: Speed,
    pub is_cyclogenesis_favorable: bool,
    pub bulk_wind_shear: Speed,
}

pub async fn resolve_tropopause(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<TropopauseDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let bond_albedo = planet.bond_albedo().unwrap_or(0.3);

    let star = find_parent_star(pool, planet.orbital_parent()).await?;
    let (star_lum, _, r_emit) =
        resolve_star_emission_profile(pool, &star, universe_epoch, at_epoch).await?;

    let system_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let positions = resolve_system_positions(pool, system_id, total_epoch).await?;

    let planet_pos =
        positions
            .get(&planet.id())
            .copied()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "planet_id".to_string(),
                reason: format!(
                    "position for planet '{}' could not be resolved",
                    planet.id()
                ),
            })?;

    let star_pos =
        positions
            .get(&star.id())
            .copied()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "star_id".to_string(),
                reason: format!("position for star '{}' could not be resolved", star.id()),
            })?;

    let orbital_distance = (planet_pos - star_pos).magnitude();
    let z_factor = if star.kind() == StarKind::BlackHole {
        gravitational_redshift_between(star.mass(), r_emit, orbital_distance)
    } else {
        1.0
    };

    let base_irradiance = orbital_irradiance(star_lum, orbital_distance);
    let top_irradiance = Irradiance::new(base_irradiance.value() / (z_factor * z_factor));

    let greenhouse = atmosphere.greenhouse_effect();

    let effective_albedo = if let Some(hydrosphere) =
        hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?
    {
        let base_eq = local_radiative_equilibrium_temperature(
            Irradiance::new(top_irradiance.value() * 0.25),
            bond_albedo,
        );
        let base_surface_temp = base_eq + greenhouse;
        let pressure = atmosphere.surface_pressure();
        let initial_state = hydrosphere.matter_state(base_surface_temp, pressure)?;
        hydrosphere.dynamic_albedo(bond_albedo, initial_state)?
    } else {
        bond_albedo
    };

    let t_eq = local_radiative_equilibrium_temperature(
        Irradiance::new(top_irradiance.value() * 0.25),
        effective_albedo,
    );

    let surf_temp = t_eq + greenhouse;
    let t_skin = grey_atmosphere_skin_temperature(t_eq);

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let dry_gamma = if env_lapse_rate.value() > 0.0 {
        env_lapse_rate
    } else {
        TemperatureGradient::new(g.value() / atm_cp.max(100.0))
    };

    let z_tropo = tropopause_altitude(surf_temp, t_skin, dry_gamma);

    Ok(TropopauseDiagnostic {
        radiative_equilibrium_temperature: t_eq,
        skin_temperature: t_skin,
        surface_temperature: surf_temp,
        tropopause_altitude: z_tropo,
        tropopause_temperature: t_skin,
        lapse_rate: dry_gamma,
    })
}

pub async fn resolve_tropopause_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<TropopauseDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let surf_temp =
        resolve_advective_surface_temperature(pool, planet_id, latitude, universe_epoch, at_epoch)
            .await?;
    let greenhouse = atmosphere.greenhouse_effect();
    let t_eq = Temperature::new((surf_temp.value() - greenhouse.value()).max(0.0));
    let t_skin = grey_atmosphere_skin_temperature(t_eq);

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let dry_gamma = if env_lapse_rate.value() > 0.0 {
        env_lapse_rate
    } else {
        TemperatureGradient::new(g.value() / atm_cp.max(100.0))
    };

    let z_tropo = tropopause_altitude(surf_temp, t_skin, dry_gamma);

    Ok(TropopauseDiagnostic {
        radiative_equilibrium_temperature: t_eq,
        skin_temperature: t_skin,
        surface_temperature: surf_temp,
        tropopause_altitude: z_tropo,
        tropopause_temperature: t_skin,
        lapse_rate: dry_gamma,
    })
}

pub async fn resolve_convective_instability(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<ConvectiveInstabilityDiagnostic> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let tropo = resolve_tropopause(pool, planet_id, universe_epoch, at_epoch).await?;
    let stratification =
        resolve_atmospheric_stratification(pool, planet_id, universe_epoch, at_epoch).await?;
    let (solvent_props, solvent_molar_mass, _) =
        resolve_condensable_species(pool, planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    let surf_temp = tropo.surface_temperature;
    let surf_press = atmosphere.surface_pressure();
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;
    let dew_point = stratification.surface_dew_point;

    let dry_gamma = dry_adiabatic_lapse_rate(g, atm_cp);
    let moist_gamma = moist_adiabatic_lapse_rate(
        g,
        atm_cp,
        surf_temp,
        surf_press,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    let profile = integrate_parcel_buoyancy_profile(
        surf_temp,
        surf_press,
        dew_point,
        atmosphere.lapse_rate(),
        scale_h,
        g,
        atm_cp,
        tropo.tropopause_altitude,
        tropo.skin_temperature,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    let lfc = level_of_free_convection(&profile, stratification.lcl_altitude);
    let el = match lfc {
        Some(z_lfc) => equilibrium_level(&profile, z_lfc),
        None => None,
    };

    let cape = match (lfc, el) {
        (Some(z_lfc), Some(z_el)) => {
            convective_available_potential_energy(&profile, z_lfc, z_el)
        }
        _ => SpecificEnergy::new(0.0),
    };

    let cin = match lfc {
        Some(z_lfc) => convective_inhibition(&profile, z_lfc),
        None => SpecificEnergy::new(0.0),
    };

    let stability = classify_atmospheric_stability(atmosphere.lapse_rate(), dry_gamma, moist_gamma);
    let morphology = classify_cloud_morphology(stability, cape);
    let is_deep = is_deep_convective_cloud(
        morphology,
        stratification.cloud_top_altitude,
        tropo.tropopause_altitude,
        cape,
    );

    Ok(ConvectiveInstabilityDiagnostic {
        surface_dew_point: dew_point,
        lcl_altitude: stratification.lcl_altitude,
        lfc_altitude: lfc,
        equilibrium_level: el,
        cape,
        cin,
        stability,
        morphology,
        is_deep_convection: is_deep,
        dry_adiabatic_lapse_rate: dry_gamma,
        moist_adiabatic_lapse_rate: moist_gamma,
    })
}

pub async fn resolve_convective_instability_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<ConvectiveInstabilityDiagnostic> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let tropo =
        resolve_tropopause_at_latitude(pool, planet_id, latitude, universe_epoch, at_epoch).await?;
    let stratification = resolve_atmospheric_stratification_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;
    let (solvent_props, solvent_molar_mass, _) =
        resolve_condensable_species(pool, planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    let surf_temp = tropo.surface_temperature;
    let surf_press = atmosphere.surface_pressure();
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;
    let dew_point = stratification.surface_dew_point;

    let dry_gamma = dry_adiabatic_lapse_rate(g, atm_cp);
    let moist_gamma = moist_adiabatic_lapse_rate(
        g,
        atm_cp,
        surf_temp,
        surf_press,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    let profile = integrate_parcel_buoyancy_profile(
        surf_temp,
        surf_press,
        dew_point,
        atmosphere.lapse_rate(),
        scale_h,
        g,
        atm_cp,
        tropo.tropopause_altitude,
        tropo.skin_temperature,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass,
    );

    let lfc = level_of_free_convection(&profile, stratification.lcl_altitude);
    let el = match lfc {
        Some(z_lfc) => equilibrium_level(&profile, z_lfc),
        None => None,
    };

    let cape = match (lfc, el) {
        (Some(z_lfc), Some(z_el)) => {
            convective_available_potential_energy(&profile, z_lfc, z_el)
        }
        _ => SpecificEnergy::new(0.0),
    };

    let cin = match lfc {
        Some(z_lfc) => convective_inhibition(&profile, z_lfc),
        None => SpecificEnergy::new(0.0),
    };

    let stability = classify_atmospheric_stability(atmosphere.lapse_rate(), dry_gamma, moist_gamma);
    let morphology = classify_cloud_morphology(stability, cape);
    let is_deep = is_deep_convective_cloud(
        morphology,
        stratification.cloud_top_altitude,
        tropo.tropopause_altitude,
        cape,
    );

    Ok(ConvectiveInstabilityDiagnostic {
        surface_dew_point: dew_point,
        lcl_altitude: stratification.lcl_altitude,
        lfc_altitude: lfc,
        equilibrium_level: el,
        cape,
        cin,
        stability,
        morphology,
        is_deep_convection: is_deep,
        dry_adiabatic_lapse_rate: dry_gamma,
        moist_adiabatic_lapse_rate: moist_gamma,
    })
}

pub async fn resolve_cloud_cover(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<CloudCoverDiagnostic> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let (solvent_props, solvent_molar_mass, surface_humidity) =
        resolve_condensable_species(pool, planet_id).await?;
    let tropo = resolve_tropopause(pool, planet_id, universe_epoch, at_epoch).await?;
    let instability =
        resolve_convective_instability(pool, planet_id, universe_epoch, at_epoch).await?;
    let stratification =
        resolve_atmospheric_stratification(pool, planet_id, universe_epoch, at_epoch).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    let surf_temp = tropo.surface_temperature;
    let surf_press = atmosphere.surface_pressure();
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let freezing_point = match &hydro_opt {
        Some(h) => h.freezing_point()?,
        None => solvent_props.normal_melting_point,
    };
    let freezing_level =
        freezing_level_altitude(surf_temp, freezing_point, env_lapse_rate).unwrap_or(Length::new(0.0));

    let (z0, z_low_top, z_mid_top, z_high_top) =
        cloud_band_altitudes(tropo.tropopause_altitude);
    let z_low_mid = Length::new(0.5 * (z0.value() + z_low_top.value()));
    let z_mid_mid = Length::new(0.5 * (z_low_top.value() + z_mid_top.value()));
    let z_high_mid = Length::new(0.5 * (z_mid_top.value() + z_high_top.value()));

    let evaluate_layer = |z_base: Length, z_top: Length, z_mid: Length| -> CloudLayerDiagnostic {
        let rh = relative_humidity_at_altitude(surface_humidity, z_mid, tropo.tropopause_altitude);
        let press_z = atmosphere.pressure_at_altitude(z_mid, scale_h);
        let temp_z = temperature_at_altitude(surf_temp, z_mid, env_lapse_rate);
        let temp_z_clamped = if temp_z.value() < tropo.skin_temperature.value() {
            tropo.skin_temperature
        } else {
            temp_z
        };

        let sigma = if surf_press.value() > 0.0 {
            (press_z.value() / surf_press.value()).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let rh_crit = layer_critical_relative_humidity(sigma);
        let c_layer = sundqvist_cloud_fraction_with_ccn(
            rh,
            rh_crit,
            atmosphere.cloud_condensation_nuclei_factor(),
        );

        let rho_air = ideal_gas_density(press_z, atm_molar_mass, temp_z_clamped);
        let r_surf = mixing_ratio_at_altitude(
            surface_humidity,
            surf_temp,
            surf_press,
            &solvent_props,
            solvent_molar_mass,
            atm_molar_mass,
        );
        let p_sat_z = saturation_vapor_pressure(temp_z_clamped, &solvent_props);
        let r_sat_z =
            saturation_mixing_ratio(p_sat_z, press_z, solvent_molar_mass, atm_molar_mass);
        let rho_condensate = adiabatic_condensate_density(rho_air, r_surf, r_sat_z);

        let ice_frac = ice_fraction_at_altitude(z_mid, freezing_level);
        let rho_liquid = liquid_condensate_density(rho_condensate, ice_frac);
        let rho_ice = ice_condensate_density(rho_condensate, ice_frac);
        let glaciation = glaciation_state_from_ice_fraction(ice_frac);

        CloudLayerDiagnostic {
            base_altitude: z_base,
            top_altitude: z_top,
            representative_altitude: z_mid,
            cloud_fraction: c_layer,
            relative_humidity: rh,
            critical_relative_humidity: rh_crit,
            liquid_water_content: rho_liquid,
            ice_water_content: rho_ice,
            ice_fraction: ice_frac,
            glaciation_state: glaciation,
        }
    };

    let low_diag = evaluate_layer(z0, z_low_top, z_low_mid);
    let mid_diag = evaluate_layer(z_low_top, z_mid_top, z_mid_mid);
    let high_diag = evaluate_layer(z_mid_top, z_high_top, z_high_mid);

    let c_combined = combine_layer_cloud_fractions_max_random_overlap(
        low_diag.cloud_fraction,
        mid_diag.cloud_fraction,
        high_diag.cloud_fraction,
    );

    let total_cloud_fraction = atmosphere.cloud_coverage_fraction().unwrap_or(c_combined);

    let classification = classify_cloud_system(
        env_lapse_rate,
        instability.dry_adiabatic_lapse_rate,
        instability.moist_adiabatic_lapse_rate,
        instability.cape,
        stratification.cloud_top_altitude,
        tropo.tropopause_altitude,
        freezing_level,
    );

    Ok(CloudCoverDiagnostic {
        total_cloud_fraction,
        low_cloud: low_diag,
        mid_cloud: mid_diag,
        high_cloud: high_diag,
        freezing_level,
        classification,
    })
}

pub async fn resolve_cloud_cover_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<CloudCoverDiagnostic> {
    let atmosphere = atmosphere_repository::get_by_planet_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let (solvent_props, solvent_molar_mass, surface_humidity) =
        resolve_condensable_species(pool, planet_id).await?;
    let tropo =
        resolve_tropopause_at_latitude(pool, planet_id, latitude, universe_epoch, at_epoch).await?;
    let instability =
        resolve_convective_instability_at_latitude(pool, planet_id, latitude, universe_epoch, at_epoch)
            .await?;
    let stratification = resolve_atmospheric_stratification_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    let surf_temp = tropo.surface_temperature;
    let surf_press = atmosphere.surface_pressure();
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let freezing_point = match &hydro_opt {
        Some(h) => h.freezing_point()?,
        None => solvent_props.normal_melting_point,
    };
    let freezing_level =
        freezing_level_altitude(surf_temp, freezing_point, env_lapse_rate).unwrap_or(Length::new(0.0));

    let (z0, z_low_top, z_mid_top, z_high_top) =
        cloud_band_altitudes(tropo.tropopause_altitude);
    let z_low_mid = Length::new(0.5 * (z0.value() + z_low_top.value()));
    let z_mid_mid = Length::new(0.5 * (z_low_top.value() + z_mid_top.value()));
    let z_high_mid = Length::new(0.5 * (z_mid_top.value() + z_high_top.value()));

    let evaluate_layer = |z_base: Length, z_top: Length, z_mid: Length| -> CloudLayerDiagnostic {
        let rh = relative_humidity_at_altitude(surface_humidity, z_mid, tropo.tropopause_altitude);
        let press_z = atmosphere.pressure_at_altitude(z_mid, scale_h);
        let temp_z = temperature_at_altitude(surf_temp, z_mid, env_lapse_rate);
        let temp_z_clamped = if temp_z.value() < tropo.skin_temperature.value() {
            tropo.skin_temperature
        } else {
            temp_z
        };

        let sigma = if surf_press.value() > 0.0 {
            (press_z.value() / surf_press.value()).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let rh_crit = layer_critical_relative_humidity(sigma);
        let c_layer = sundqvist_cloud_fraction_with_ccn(
            rh,
            rh_crit,
            atmosphere.cloud_condensation_nuclei_factor(),
        );

        let rho_air = ideal_gas_density(press_z, atm_molar_mass, temp_z_clamped);
        let r_surf = mixing_ratio_at_altitude(
            surface_humidity,
            surf_temp,
            surf_press,
            &solvent_props,
            solvent_molar_mass,
            atm_molar_mass,
        );
        let p_sat_z = saturation_vapor_pressure(temp_z_clamped, &solvent_props);
        let r_sat_z =
            saturation_mixing_ratio(p_sat_z, press_z, solvent_molar_mass, atm_molar_mass);
        let rho_condensate = adiabatic_condensate_density(rho_air, r_surf, r_sat_z);

        let ice_frac = ice_fraction_at_altitude(z_mid, freezing_level);
        let rho_liquid = liquid_condensate_density(rho_condensate, ice_frac);
        let rho_ice = ice_condensate_density(rho_condensate, ice_frac);
        let glaciation = glaciation_state_from_ice_fraction(ice_frac);

        CloudLayerDiagnostic {
            base_altitude: z_base,
            top_altitude: z_top,
            representative_altitude: z_mid,
            cloud_fraction: c_layer,
            relative_humidity: rh,
            critical_relative_humidity: rh_crit,
            liquid_water_content: rho_liquid,
            ice_water_content: rho_ice,
            ice_fraction: ice_frac,
            glaciation_state: glaciation,
        }
    };

    let low_diag = evaluate_layer(z0, z_low_top, z_low_mid);
    let mid_diag = evaluate_layer(z_low_top, z_mid_top, z_mid_mid);
    let high_diag = evaluate_layer(z_mid_top, z_high_top, z_high_mid);

    let c_combined = combine_layer_cloud_fractions_max_random_overlap(
        low_diag.cloud_fraction,
        mid_diag.cloud_fraction,
        high_diag.cloud_fraction,
    );

    let total_cloud_fraction = atmosphere.cloud_coverage_fraction().unwrap_or(c_combined);

    let classification = classify_cloud_system(
        env_lapse_rate,
        instability.dry_adiabatic_lapse_rate,
        instability.moist_adiabatic_lapse_rate,
        instability.cape,
        stratification.cloud_top_altitude,
        tropo.tropopause_altitude,
        freezing_level,
    );

    Ok(CloudCoverDiagnostic {
        total_cloud_fraction,
        low_cloud: low_diag,
        mid_cloud: mid_diag,
        high_cloud: high_diag,
        freezing_level,
        classification,
    })
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

    let v_pot = if ocean_cov <= 0.0 || surf_temp.value() >= solvent_props.critical_temperature.value() {
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