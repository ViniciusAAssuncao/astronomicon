use crate::climate::temperature::{
    resolve_advective_surface_temperature, resolve_global_mean_temperature,
    resolve_latitudinal_surface_temperature,
};
use crate::error::AppResult;
use astronomicon_core::domain::Planet;
use astronomicon_core::error::DomainError;
use astronomicon_core::math::circulation::{
    circulation_cells_per_hemisphere, equatorial_rossby_deformation_radius, rhines_scale,
};
use astronomicon_core::math::climate::{
    atmospheric_column_heat_capacity, combined_column_heat_capacity,
    combined_thermal_redistribution_efficiency, thermal_redistribution_efficiency,
};
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::math::rotation::{
    angular_velocity_from_rotation_period, coriolis_parameter, rossby_beta_parameter,
};
use astronomicon_core::math::wind::{
    latitudinal_temperature_gradient, surface_wind_components, surface_wind_speed,
    zonal_jet_stream_speed,
};
use astronomicon_core::units::{
    Angle, AngularVelocity, Duration, Length, Speed, TemperatureGradient,
};
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlanetaryCirculationDiagnostic {
    pub angular_velocity: AngularVelocity,
    pub equatorial_beta: f64,
    pub rossby_deformation_radius: Length,
    pub rhines_scale: Length,
    pub circulation_cells: u32,
    pub column_heat_capacity: f64,
    pub thermal_redistribution_efficiency: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindProfileDiagnostic {
    pub latitude: Angle,
    pub coriolis_parameter: AngularVelocity,
    pub temperature_gradient: TemperatureGradient,
    pub jet_stream_speed: Speed,
    pub surface_wind_speed: Speed,
    pub surface_wind_u: Speed,
    pub surface_wind_v: Speed,
}

pub async fn resolve_planetary_circulation(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<PlanetaryCirculationDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let radius = planet
        .equatorial_radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "equatorial_radius".to_string(),
            reason: "planet does not have equatorial radius".to_string(),
        })?;

    let rot_period = planet
        .rotation_period()
        .unwrap_or_else(|| Duration::new(86400.0));
    let omega = angular_velocity_from_rotation_period(rot_period);
    let beta_eq = rossby_beta_parameter(omega, Angle::new(0.0), radius);

    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, radius);

    let global_mean =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;

    let (scale_h, atm_col_heat_cap) = if let Some(atmosphere) =
        atmosphere_repository::get_by_planet_id(pool, &planet_id).await?
    {
        let h = atmosphere.scale_height(g, global_mean)?;
        let cp_gas = atmosphere.mean_specific_heat_capacity()?;
        let c_p = atmospheric_column_heat_capacity(atmosphere.surface_pressure(), g, cp_gas);
        (h, c_p)
    } else {
        (Length::new(8500.0), 0.0)
    };

    let lr = equatorial_rossby_deformation_radius(g, scale_h, beta_eq);

    let temp_eq = resolve_latitudinal_surface_temperature(
        pool,
        planet_id,
        Angle::new(0.0),
        universe_epoch,
        at_epoch,
    )
    .await?;
    let temp_pole = resolve_latitudinal_surface_temperature(
        pool,
        planet_id,
        Angle::new(PI / 2.0),
        universe_epoch,
        at_epoch,
    )
    .await?;

    let delta_t = (temp_eq.value() - temp_pole.value()).abs().max(1.0);
    let char_u = Speed::new((g.value() * scale_h.value() * (delta_t / global_mean.value())).sqrt());
    let l_beta = rhines_scale(char_u, beta_eq);

    let cells = circulation_cells_per_hemisphere(radius, l_beta);

    let (efficiency, col_heat_cap) = if let Some(hydrosphere) =
        hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?
    {
        let oce_col_heat_cap = hydrosphere.oceanic_column_heat_capacity()?;
        let cov = hydrosphere.surface_coverage_fraction();
        let eff = combined_thermal_redistribution_efficiency(
            atm_col_heat_cap,
            oce_col_heat_cap,
            cov,
            cells,
        );
        let comb_cap = combined_column_heat_capacity(atm_col_heat_cap, oce_col_heat_cap, cov);
        (eff, comb_cap)
    } else {
        let eff = thermal_redistribution_efficiency(atm_col_heat_cap, cells);
        (eff, atm_col_heat_cap)
    };

    Ok(PlanetaryCirculationDiagnostic {
        angular_velocity: omega,
        equatorial_beta: beta_eq,
        rossby_deformation_radius: lr,
        rhines_scale: l_beta,
        circulation_cells: cells,
        column_heat_capacity: col_heat_cap,
        thermal_redistribution_efficiency: efficiency,
    })
}

pub async fn resolve_wind_profile_at_latitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<WindProfileDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let radius = planet
        .equatorial_radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "equatorial_radius".to_string(),
            reason: "planet does not have equatorial radius".to_string(),
        })?;

    let rot_period = planet
        .rotation_period()
        .unwrap_or_else(|| Duration::new(86400.0));
    let omega = angular_velocity_from_rotation_period(rot_period);
    let f = coriolis_parameter(omega, latitude);

    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, radius);

    let d_phi = (1.0 * PI) / 180.0;
    let lat_n = Angle::new((latitude.value() + d_phi).min(PI / 2.0));
    let lat_s = Angle::new((latitude.value() - d_phi).max(-PI / 2.0));

    let t_n = resolve_advective_surface_temperature(
        pool,
        planet_id,
        lat_n,
        universe_epoch,
        at_epoch,
    )
    .await?;
    let t_s = resolve_advective_surface_temperature(
        pool,
        planet_id,
        lat_s,
        universe_epoch,
        at_epoch,
    )
    .await?;
    let t_local = resolve_advective_surface_temperature(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch,
    )
    .await?;

    let t_grad = latitudinal_temperature_gradient(t_n, t_s, lat_n, lat_s, radius);

    let scale_h = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atm) => atm.scale_height(g, t_local)?,
        None => Length::new(8500.0),
    };

    let u_jet = zonal_jet_stream_speed(g, omega, radius, latitude, t_local, t_grad, scale_h);
    let friction_factor = 0.65;
    let cross_angle = Angle::new((20.0 * PI) / 180.0);

    let u_surf = surface_wind_speed(u_jet, friction_factor);
    let (u_surf_x, u_surf_y) = surface_wind_components(u_jet, friction_factor, cross_angle);

    Ok(WindProfileDiagnostic {
        latitude,
        coriolis_parameter: f,
        temperature_gradient: t_grad,
        jet_stream_speed: u_jet,
        surface_wind_speed: u_surf,
        surface_wind_u: u_surf_x,
        surface_wind_v: u_surf_y,
    })
}
