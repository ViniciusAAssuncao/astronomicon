use crate::climate::atmosphere::{
    AtmosphericStratificationDiagnostic, resolve_atmospheric_stratification_at_latitude,
};
use crate::climate::circulation::{WindProfileDiagnostic, resolve_wind_profile_at_latitude};
use crate::climate::clouds::cover::{CloudCoverDiagnostic, resolve_cloud_cover_at_latitude};
use crate::climate::clouds::instability::{
    ConvectiveInstabilityDiagnostic, resolve_convective_instability_at_latitude,
};
use crate::climate::clouds::tropopause::{
    TropopauseDiagnostic, resolve_tropopause_at_latitude,
};
use crate::climate::condensable_species::{
    resolve_all_condensable_species, resolve_condensable_species,
};
use crate::error::AppResult;
use astronomicon_core::chemistry::solvent::SolventProperties;
use astronomicon_core::domain::{Atmosphere, Planet};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::aerosol::particle_terminal_velocity;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::climate::temperature_at_altitude;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::precipitation::{
    AcidityClassification, CondensatePrimaryClass, PrecipitationPhase,
    SurfaceCondensationType, apply_bergeron_enhancement, bergeron_enhancement_factor,
    classify_surface_condensation, evaluate_precipitation_acidity,
    layer_vertical_velocity_scale, resolve_sedimentation_balance,
    scan_precipitation_phase, subcloud_evaporation_profile,
};
use astronomicon_core::units::{
    Acceleration, Angle, Density, Duration, DynamicViscosity, Length, MassFluxDensity,
    MolarMass, Speed, Temperature,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrecipitationDiagnostic {
    pub phase: PrecipitationPhase,
    pub primary_class: CondensatePrimaryClass,
    pub mass_flux: MassFluxDensity,
    pub linear_accumulation_rate: Speed,
    pub reaches_surface: bool,
    pub acidity: AcidityClassification,
    pub ph: Option<f64>,
    pub surface_condensation: SurfaceCondensationType,
}

pub fn calculate_precipitation_diagnostic(
    atmosphere: &Atmosphere,
    cloud_diag: &CloudCoverDiagnostic,
    stratification: &AtmosphericStratificationDiagnostic,
    tropo: &TropopauseDiagnostic,
    instability: &ConvectiveInstabilityDiagnostic,
    wind: &WindProfileDiagnostic,
    solvent_props: &SolventProperties,
    solvent_molar_mass: MolarMass,
    surface_humidity: f64,
    freezing_point: Temperature,
    scale_h: Length,
    gravity: Acceleration,
    hydro_composition: &[(String, f64)],
) -> AppResult<PrecipitationDiagnostic> {
    let surf_temp = tropo.surface_temperature;
    let surf_press = atmosphere.surface_pressure();
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let bulk_shear = (wind.jet_stream_speed.value() - wind.surface_wind_speed.value()).abs();
    let vertical_wind_shear = bulk_shear / tropo.tropopause_altitude.value().max(1.0);

    let layers = [
        &cloud_diag.high_cloud,
        &cloud_diag.mid_cloud,
        &cloud_diag.low_cloud,
    ];

    let mut current_flux = 0.0;
    let mut current_droplet_radius = Length::new(0.0);

    for layer in layers {
        let layer_thickness =
            Length::new((layer.top_altitude.value() - layer.base_altitude.value()).max(0.0));
        let rho_cond =
            Density::new(layer.liquid_water_content.value() + layer.ice_water_content.value());
        let ice_frac = layer.ice_fraction.clamp(0.0, 1.0);
        let rho_p_val = ((1.0 - ice_frac) * solvent_props.liquid_density.value()
            + ice_frac * solvent_props.solid_density.value())
        .max(1.0);
        let particle_density = Density::new(rho_p_val);

        let temp_z =
            temperature_at_altitude(surf_temp, layer.representative_altitude, env_lapse_rate);
        let press_z = atmosphere.pressure_at_altitude(layer.representative_altitude, scale_h);
        let fluid_density = ideal_gas_density(press_z, atm_molar_mass, temp_z);
        let dynamic_viscosity = atmosphere
            .mean_dynamic_viscosity(temp_z)
            .unwrap_or_else(|_| DynamicViscosity::new(1.81e-5));

        let w_scale = layer_vertical_velocity_scale(
            instability.morphology,
            instability.cape,
            layer_thickness,
            vertical_wind_shear,
        );

        let sed_res = resolve_sedimentation_balance(
            rho_cond,
            particle_density,
            fluid_density,
            dynamic_viscosity,
            gravity,
            w_scale,
            atmosphere.cloud_condensation_nuclei_factor(),
        );

        let b_factor = bergeron_enhancement_factor(
            layer.glaciation_state,
            layer.ice_fraction,
            temp_z,
            solvent_props,
        );

        let sed_frac = apply_bergeron_enhancement(sed_res.sedimentable_fraction, b_factor);

        let droplet_radius = if sed_res.sedimentable_fraction > 0.05 {
            Length::new(
                sed_res
                    .critical_radius
                    .value()
                    .max(sed_res.mean_droplet_radius.value()),
            )
        } else if sed_res.mean_droplet_radius.value() > 0.0 {
            sed_res.mean_droplet_radius
        } else {
            Length::new(10.0e-6)
        };

        let v_fall = particle_terminal_velocity(
            gravity,
            particle_density,
            fluid_density,
            droplet_radius,
            dynamic_viscosity,
        )
        .value()
        .max(w_scale.value())
        .max(0.1);

        let gen_flux = sed_frac * rho_cond.value() * v_fall * layer.cloud_fraction;

        current_flux += gen_flux;
        if gen_flux > 0.0 || current_droplet_radius.value() <= 0.0 {
            current_droplet_radius = droplet_radius;
        }
    }

    let cloud_base = cloud_diag.low_cloud.base_altitude;
    let phase = scan_precipitation_phase(surf_temp, cloud_base, env_lapse_rate, freezing_point);
    let surface_condensation =
        classify_surface_condensation(stratification.surface_dew_point, freezing_point);

    let atm_comp: Vec<(String, f64)> = atmosphere
        .composition()
        .iter()
        .map(|c| (c.formula().to_string(), c.percentage()))
        .collect();

    let acidity_diag = evaluate_precipitation_acidity(
        hydro_composition,
        &atm_comp,
        surf_press,
        surf_temp,
    );

    let dyn_visc_surf = atmosphere
        .mean_dynamic_viscosity(surf_temp)
        .unwrap_or_else(|_| DynamicViscosity::new(1.81e-5));

    let (final_flux, reaches_surface) =
        if current_flux > 0.0 && current_droplet_radius.value() > 0.0 {
            let evap_res = subcloud_evaporation_profile(
                current_droplet_radius,
                cloud_base,
                surf_temp,
                surf_press,
                surface_humidity,
                env_lapse_rate,
                scale_h,
                tropo.tropopause_altitude,
                gravity,
                dyn_visc_surf,
                solvent_props,
                solvent_molar_mass,
                atm_molar_mass,
                phase,
            );

            let remaining_frac = evap_res.mass_fraction_remaining;
            let flux_at_surf = current_flux * remaining_frac;
            let hits = !evap_res.is_virga && flux_at_surf > 1e-9;
            (if hits { flux_at_surf } else { 0.0 }, hits)
        } else {
            (0.0, false)
        };

    let rho_cond = match phase {
        PrecipitationPhase::Solid => solvent_props.solid_density.value(),
        _ => solvent_props.liquid_density.value(),
    };

    let speed_val = if rho_cond > 0.0 && reaches_surface {
        final_flux / rho_cond
    } else {
        0.0
    };

    Ok(PrecipitationDiagnostic {
        phase,
        primary_class: acidity_diag.primary_class(),
        mass_flux: MassFluxDensity::new(final_flux),
        linear_accumulation_rate: Speed::new(speed_val),
        reaches_surface,
        acidity: acidity_diag.acidity_classification(),
        ph: acidity_diag.ph(),
        surface_condensation,
    })
}

pub async fn resolve_precipitation_diagnostic(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<PrecipitationDiagnostic> {
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
    let instability = resolve_convective_instability_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;
    let stratification = resolve_atmospheric_stratification_at_latitude(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;
    let cloud_diag =
        resolve_cloud_cover_at_latitude(pool, planet_id, latitude, universe_epoch, at_epoch).await?;
    let wind_diag =
        resolve_wind_profile_at_latitude(pool, planet_id, latitude, universe_epoch, at_epoch)
            .await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);
    let scale_h = atmosphere.scale_height(g, tropo.surface_temperature)?;

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let freezing_point = match &hydro_opt {
        Some(h) => h.freezing_point()?,
        None => solvent_props.normal_melting_point,
    };

    let hydro_composition: Vec<(String, f64)> = if let Some(ref h) = hydro_opt {
        h.composition()
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect()
    } else {
        let all = resolve_all_condensable_species(pool, planet_id).await?;
        if all.is_empty() {
            vec![("H2O".to_string(), 100.0)]
        } else {
            atmosphere
                .composition()
                .iter()
                .filter(|c| {
                    astronomicon_core::chemistry::solvent_properties_of(c.formula()).is_some()
                })
                .map(|c| (c.formula().to_string(), c.percentage()))
                .collect()
        }
    };

    calculate_precipitation_diagnostic(
        &atmosphere,
        &cloud_diag,
        &stratification,
        &tropo,
        &instability,
        &wind_diag,
        &solvent_props,
        solvent_molar_mass,
        surface_humidity,
        freezing_point,
        scale_h,
        g,
        &hydro_composition,
    )
}

pub async fn resolve_precipitation_diagnostic_global(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<PrecipitationDiagnostic> {
    resolve_precipitation_diagnostic(
        pool,
        planet_id,
        Angle::new(0.0),
        universe_epoch,
        at_epoch,
    )
    .await
}