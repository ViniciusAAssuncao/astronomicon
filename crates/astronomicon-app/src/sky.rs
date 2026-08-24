pub use color_processing::*;
pub use profiles::*;
pub use scattering_summary::*;
pub use spectral_integration::*;

use crate::climate::{
    CloudCoverDiagnostic, resolve_atmospheric_stratification, resolve_cloud_cover,
    resolve_condensable_species, resolve_convective_instability,
    resolve_global_mean_temperature, resolve_star_emission_profile, resolve_tropopause,
    resolve_wind_profile_at_latitude,
};
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use crate::volcanism::resolve_planetary_volcanism;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::climate::temperature_at_altitude;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::precipitation::{
    layer_vertical_velocity_scale, resolve_sedimentation_balance,
};
use astronomicon_core::math::radiometry::{stellar_angular_radius, stellar_solid_angle};
use astronomicon_core::math::scattering::MultipleScatteringConfig;
use astronomicon_core::units::{
    Angle, ColorRGB, Density, Duration, DynamicViscosity, Length, Vector3,
};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScatteringCoefficients {
    pub rayleigh_r: f64,
    pub rayleigh_g: f64,
    pub rayleigh_b: f64,
    pub mie_r: f64,
    pub mie_g: f64,
    pub mie_b: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyColorDiagnostic {
    pub zenith_color: ColorRGB,
    pub horizon_color: ColorRGB,
    pub sunset_color: ColorRGB,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyDiagnostic {
    pub scattering: ScatteringCoefficients,
    pub colors: SkyColorDiagnostic,
    pub total_optical_depth_r: f64,
    pub total_optical_depth_g: f64,
    pub total_optical_depth_b: f64,
    pub clouds: CloudCoverDiagnostic,
}

pub async fn resolve_sky_diagnostics(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Option<SkyDiagnostic>> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let atm_row = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?;
    let atm = match atm_row {
        Some(a) => a,
        None => return Ok(None),
    };

    let star = find_parent_star(pool, planet.orbital_parent()).await?;
    let (_, star_temp, r_emit) =
        resolve_star_emission_profile(pool, &star, universe_epoch, at_epoch).await?;

    let sys_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;
    let positions = resolve_system_positions(pool, sys_id, universe_epoch + at_epoch).await?;
    let pos_p = positions
        .get(&planet_id)
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: "planet position could not be resolved".to_string(),
        })?;
    let pos_s = positions
        .get(&star.id())
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: "star position could not be resolved".to_string(),
        })?;
    let distance = (*pos_p - *pos_s).magnitude();

    let star_angular_radius = stellar_angular_radius(r_emit, distance);
    let solid_angle_sun = stellar_solid_angle(star_angular_radius).value();

    let surf_temp =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let wind_diag = resolve_wind_profile_at_latitude(
        pool,
        planet_id,
        Angle::new(0.0),
        universe_epoch,
        at_epoch,
    )
    .await?;
    let volc_diag = resolve_planetary_volcanism(pool, planet_id, universe_epoch, at_epoch).await?;
    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let cloud_diag = resolve_cloud_cover(pool, planet_id, universe_epoch, at_epoch).await?;
    let strat_diag =
        resolve_atmospheric_stratification(pool, planet_id, universe_epoch, at_epoch).await?;
    let tropo = resolve_tropopause(pool, planet_id, universe_epoch, at_epoch).await?;
    let instability =
        resolve_convective_instability(pool, planet_id, universe_epoch, at_epoch).await?;
    let (solvent_props, _, _) = resolve_condensable_species(pool, planet_id).await?;

    let eq_radius = planet
        .equatorial_radius()
        .unwrap_or_else(|| Length::new(6371e3));
    let g = surface_gravity(gravitational_parameter(planet.mass()), eq_radius);
    let ocean_cov = hydro_opt
        .as_ref()
        .map(|h| h.surface_coverage_fraction())
        .unwrap_or(0.0);

    let bulk_shear = (wind_diag.jet_stream_speed.value() - wind_diag.surface_wind_speed.value()).abs();
    let vertical_wind_shear = bulk_shear / tropo.tropopause_altitude.value().max(1.0);
    let scale_h = atm.scale_height(g, surf_temp)?;
    let atm_molar_mass = atm.mean_molar_mass()?;

    let compute_layer_droplet_radius =
        |layer: &crate::climate::clouds::layer::CloudLayerDiagnostic| -> Length {
            let dz =
                Length::new((layer.top_altitude.value() - layer.base_altitude.value()).max(0.0));
            let rho_cond = Density::new(
                layer.liquid_water_content.value() + layer.ice_water_content.value(),
            );
            let ice_frac = layer.ice_fraction.clamp(0.0, 1.0);
            let rho_p_val = ((1.0 - ice_frac) * solvent_props.liquid_density.value()
                + ice_frac * solvent_props.solid_density.value())
            .max(1.0);
            let particle_density = Density::new(rho_p_val);

            let temp_z = temperature_at_altitude(
                surf_temp,
                layer.representative_altitude,
                atm.lapse_rate(),
            );
            let press_z = atm.pressure_at_altitude(layer.representative_altitude, scale_h);
            let fluid_density = ideal_gas_density(press_z, atm_molar_mass, temp_z);
            let dynamic_viscosity = atm
                .mean_dynamic_viscosity(temp_z)
                .unwrap_or_else(|_| DynamicViscosity::new(1.81e-5));

            let w_scale = layer_vertical_velocity_scale(
                instability.morphology,
                instability.cape,
                dz,
                vertical_wind_shear,
            );

            let sed_res = resolve_sedimentation_balance(
                rho_cond,
                particle_density,
                fluid_density,
                dynamic_viscosity,
                g,
                w_scale,
                atm.cloud_condensation_nuclei_factor(),
            );

            if sed_res.sedimentable_fraction > 0.05 {
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
            }
        };

    let r_low = compute_layer_droplet_radius(&cloud_diag.low_cloud);
    let r_mid = compute_layer_droplet_radius(&cloud_diag.mid_cloud);
    let r_high = compute_layer_droplet_radius(&cloud_diag.high_cloud);

    let mut effective_radius = r_low;
    let mut max_weight = -1.0;
    for (layer, r_l) in [
        (&cloud_diag.low_cloud, r_low),
        (&cloud_diag.mid_cloud, r_mid),
        (&cloud_diag.high_cloud, r_high),
    ] {
        let weight = (layer.liquid_water_content.value() + layer.ice_water_content.value())
            * layer.cloud_fraction;
        if weight > max_weight {
            max_weight = weight;
            effective_radius = r_l;
        }
    }

    let (atmosphere, dust_profile, volcanic_profile, opt_props, scale_h) = build_sky_atmosphere(
        &planet,
        &atm,
        surf_temp,
        wind_diag.surface_wind_speed,
        &volc_diag,
        ocean_cov,
        eq_radius,
        g,
        &cloud_diag,
        &strat_diag,
        &solvent_props,
        effective_radius,
    )?;

    let ground_albedo = planet.bond_albedo().unwrap_or(0.15);
    let ms_config =
        MultipleScatteringConfig::new(32, 16, Length::new(100_000.0), ground_albedo, 1.0);

    let radiances = integrate_sky_spectrum(
        &atmosphere,
        &ms_config,
        star_temp,
        solid_angle_sun,
        star_angular_radius,
        eq_radius,
        scale_h,
        opt_props.refractivity_stp(),
        surf_temp,
        atm.surface_pressure(),
    );

    let colors = process_sky_colors(
        radiances.xyz_zenith,
        radiances.xyz_horizon,
        radiances.xyz_sunset,
        radiances.xyz_sun_toa,
    );

    let ray_origin = Vector3::new(0.0, eq_radius.value(), 0.0);
    let summary = resolve_scattering_summary(
        &atmosphere,
        &dust_profile,
        &volcanic_profile,
        &opt_props,
        atm.surface_pressure(),
        surf_temp,
        ray_origin,
    );

    Ok(Some(SkyDiagnostic {
        scattering: summary.scattering,
        colors,
        total_optical_depth_r: summary.total_optical_depth_r,
        total_optical_depth_g: summary.total_optical_depth_g,
        total_optical_depth_b: summary.total_optical_depth_b,
        clouds: cloud_diag,
    }))
}