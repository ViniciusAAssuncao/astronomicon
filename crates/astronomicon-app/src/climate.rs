use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use astronomicon_core::domain::{Barycenter, BarycenterMember, OrbitalParent, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::circulation::{
    circulation_cells_per_hemisphere, equatorial_rossby_deformation_radius, rhines_scale,
};
use astronomicon_core::math::climate::{
    advective_local_temperature, atmospheric_column_heat_capacity, blended_local_temperature,
    day_length_half_angle, local_radiative_equilibrium_temperature, mean_daily_insolation_factor,
    solar_declination, temperature_at_altitude, thermal_redistribution_efficiency,
};
use astronomicon_core::math::gravity::{
    combined_gravitational_parameter, gravitational_parameter, surface_gravity,
};
use astronomicon_core::math::kepler::true_anomaly_at_epoch;
use astronomicon_core::math::radiometry::{
    equilibrium_temperature, escape_velocity, orbital_irradiance, stellar_luminosity,
};
use astronomicon_core::math::rotation::{
    angular_velocity_from_rotation_period, coriolis_parameter, rossby_beta_parameter,
};
use astronomicon_core::math::stellar_wind::{
    reimers_mass_loss_rate, stellar_wind_density, stellar_wind_dynamic_pressure,
    terminal_wind_speed,
};
use astronomicon_core::math::wind::{
    latitudinal_temperature_gradient, surface_wind_components, surface_wind_speed,
    zonal_jet_stream_speed,
};
use astronomicon_core::units::{
    Angle, AngularVelocity, Density, Duration, Length, MassRate, Pressure, Speed, Temperature,
    TemperatureGradient,
};
use astronomicon_db::repositories::{
    atmosphere_repository, barycenter_repository, planet_repository, star_repository,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StellarWindDiagnostic {
    pub mass_loss_rate: MassRate,
    pub escape_velocity: Speed,
    pub terminal_wind_speed: Speed,
    pub wind_density_at_orbit: Density,
    pub dynamic_pressure: Pressure,
}

async fn collect_stars_from_barycenter(
    pool: &SqlitePool,
    barycenter_id: &Uuid,
    visited: &mut std::collections::HashSet<Uuid>,
) -> AppResult<Vec<Star>> {
    if !visited.insert(*barycenter_id) {
        return Err(DomainError::InvalidInvariant {
            field: "barycenter".to_string(),
            reason: format!("circular reference detected in barycenter '{}'", barycenter_id),
        }
        .into());
    }

    let row = barycenter_repository::get_by_id(pool, barycenter_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "barycenter_id".to_string(),
            reason: format!("barycenter '{}' not found", barycenter_id),
        })?;
    let barycenter = Barycenter::try_from(row)?;

    let mut stars = Vec::new();

    for member in [barycenter.member_primary(), barycenter.member_secondary()] {
        match member {
            BarycenterMember::Star(star_id) => {
                let star_row = star_repository::get_by_id(pool, &star_id)
                    .await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "star_id".to_string(),
                        reason: format!("star '{}' in barycenter not found", star_id),
                    })?;
                stars.push(Star::try_from(star_row)?);
            }
            BarycenterMember::Planet(_) => {}
            BarycenterMember::Barycenter(sub_id) => {
                let mut sub_stars =
                    Box::pin(collect_stars_from_barycenter(pool, &sub_id, visited)).await?;
                stars.append(&mut sub_stars);
            }
        }
    }

    visited.remove(barycenter_id);
    Ok(stars)
}

async fn find_parent_star(pool: &SqlitePool, planet: &Planet) -> AppResult<Star> {
    let mut current_parent = planet.orbital_parent();
    let mut visited_barycenters = std::collections::HashSet::new();

    loop {
        match current_parent {
            OrbitalParent::Star(star_id) => {
                let row = star_repository::get_by_id(pool, &star_id)
                    .await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_star_id".to_string(),
                        reason: format!("parent star '{}' not found", star_id),
                    })?;
                return Ok(Star::try_from(row)?);
            }
            OrbitalParent::Planet(planet_id) => {
                let row = planet_repository::get_by_id(pool, &planet_id)
                    .await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_planet_id".to_string(),
                        reason: format!("parent planet '{}' not found", planet_id),
                    })?;
                let parent_planet = Planet::try_from(row)?;
                current_parent = parent_planet.orbital_parent();
            }
            OrbitalParent::Barycenter(barycenter_id) => {
                let stars =
                    collect_stars_from_barycenter(pool, &barycenter_id, &mut visited_barycenters)
                        .await?;
                let most_massive = stars
                    .into_iter()
                    .max_by(|a, b| {
                        a.mass()
                            .partial_cmp(&b.mass())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "barycenter_stars".to_string(),
                        reason: format!(
                            "no stars found in barycenter '{}' hierarchy",
                            barycenter_id
                        ),
                    })?;
                return Ok(most_massive);
            }
            OrbitalParent::Fixed => {
                return Err(DomainError::InvalidInvariant {
                    field: "planet_hierarchy".to_string(),
                    reason: "planet has no parent star in hierarchy".to_string(),
                }
                .into());
            }
        }
    }
}

pub async fn resolve_global_mean_temperature(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Temperature> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let bond_albedo = planet
        .bond_albedo()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "bond_albedo".to_string(),
            reason: "planet does not have bond albedo".to_string(),
        })?;

    let star = find_parent_star(pool, &planet).await?;

    let star_temp = star
        .effective_temperature()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "effective_temperature".to_string(),
            reason: "star does not have effective temperature".to_string(),
        })?;

    let star_radius = star
        .radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: "star does not have radius".to_string(),
        })?;

    let system_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let positions = resolve_system_positions(pool, system_id, total_epoch).await?;

    let planet_pos = positions
        .get(&planet.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("position for planet '{}' could not be resolved", planet.id()),
        })?;

    let star_pos = positions
        .get(&star.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("position for star '{}' could not be resolved", star.id()),
        })?;

    let orbital_distance = (planet_pos - star_pos).magnitude();

    let eq_temp = equilibrium_temperature(star_temp, star_radius, orbital_distance, bond_albedo);

    let greenhouse = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atmosphere) => atmosphere.greenhouse_effect(),
        None => Temperature::new(0.0),
    };

    Ok(eq_temp + greenhouse)
}

pub async fn resolve_latitudinal_surface_temperature(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Temperature> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, &planet).await?;

    let thermal_inertia = planet
        .thermal_inertia()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "thermal_inertia".to_string(),
            reason: "planet does not have thermal inertia".to_string(),
        })?;

    let obliquity = planet
        .obliquity()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "obliquity".to_string(),
            reason: "planet does not have obliquity".to_string(),
        })?;

    let solstice_true_anomaly =
        planet
            .solstice_true_anomaly()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "solstice_true_anomaly".to_string(),
                reason: "planet does not have solstice true anomaly".to_string(),
            })?;

    let orbital_elements =
        planet
            .orbital_elements()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "planet does not have orbital elements".to_string(),
            })?;

    let bond_albedo = planet
        .bond_albedo()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "bond_albedo".to_string(),
            reason: "planet does not have bond albedo".to_string(),
        })?;

    let star_temp = star
        .effective_temperature()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "effective_temperature".to_string(),
            reason: "star does not have effective temperature".to_string(),
        })?;

    let star_radius = star
        .radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: "star does not have radius".to_string(),
        })?;

    let system_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let mu = combined_gravitational_parameter(planet.mass(), star.mass());
    let true_anomaly = true_anomaly_at_epoch(&orbital_elements, mu, total_epoch)?;

    let declination = solar_declination(
        obliquity,
        orbital_elements.argument_of_periapsis(),
        solstice_true_anomaly,
        true_anomaly,
    );
    let half_angle = day_length_half_angle(latitude, declination);
    let insolation_factor = mean_daily_insolation_factor(latitude, declination, half_angle);

    let positions = resolve_system_positions(pool, system_id, total_epoch).await?;
    let planet_pos = positions
        .get(&planet.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("position for planet '{}' could not be resolved", planet.id()),
        })?;

    let star_pos = positions
        .get(&star.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("position for star '{}' could not be resolved", star.id()),
        })?;

    let orbital_distance = (planet_pos - star_pos).magnitude();
    let star_lum = stellar_luminosity(star_radius, star_temp);
    let top_irradiance = orbital_irradiance(star_lum, orbital_distance);
    let local_insolation = top_irradiance * insolation_factor;

    let local_eq = local_radiative_equilibrium_temperature(local_insolation, bond_albedo);
    let global_mean =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let blended = blended_local_temperature(global_mean, local_eq, thermal_inertia);

    Ok(blended)
}

pub async fn resolve_advective_surface_temperature(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Temperature> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, &planet).await?;

    let obliquity = planet
        .obliquity()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "obliquity".to_string(),
            reason: "planet does not have obliquity".to_string(),
        })?;

    let solstice_true_anomaly =
        planet
            .solstice_true_anomaly()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "solstice_true_anomaly".to_string(),
                reason: "planet does not have solstice true anomaly".to_string(),
            })?;

    let orbital_elements =
        planet
            .orbital_elements()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "planet does not have orbital elements".to_string(),
            })?;

    let bond_albedo = planet
        .bond_albedo()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "bond_albedo".to_string(),
            reason: "planet does not have bond albedo".to_string(),
        })?;

    let star_temp = star
        .effective_temperature()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "effective_temperature".to_string(),
            reason: "star does not have effective temperature".to_string(),
        })?;

    let star_radius = star
        .radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: "star does not have radius".to_string(),
        })?;

    let system_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let mu = combined_gravitational_parameter(planet.mass(), star.mass());
    let true_anomaly = true_anomaly_at_epoch(&orbital_elements, mu, total_epoch)?;

    let declination = solar_declination(
        obliquity,
        orbital_elements.argument_of_periapsis(),
        solstice_true_anomaly,
        true_anomaly,
    );
    let half_angle = day_length_half_angle(latitude, declination);
    let insolation_factor = mean_daily_insolation_factor(latitude, declination, half_angle);

    let positions = resolve_system_positions(pool, system_id, total_epoch).await?;
    let planet_pos = positions
        .get(&planet.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("position for planet '{}' could not be resolved", planet.id()),
        })?;

    let star_pos = positions
        .get(&star.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("position for star '{}' could not be resolved", star.id()),
        })?;

    let orbital_distance = (planet_pos - star_pos).magnitude();
    let star_lum = stellar_luminosity(star_radius, star_temp);
    let top_irradiance = orbital_irradiance(star_lum, orbital_distance);
    let local_insolation = top_irradiance * insolation_factor;

    let local_eq = local_radiative_equilibrium_temperature(local_insolation, bond_albedo);
    let global_mean =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;

    let circulation =
        resolve_planetary_circulation(pool, planet_id, universe_epoch, at_epoch).await?;
    let advective = advective_local_temperature(
        global_mean,
        local_eq,
        circulation.thermal_redistribution_efficiency,
    );

    Ok(advective)
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

    let (scale_h, col_heat_cap) =
        if let Some(atmosphere) = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?
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
    let efficiency = thermal_redistribution_efficiency(col_heat_cap, cells);

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

    let t_n =
        resolve_advective_surface_temperature(pool, planet_id, lat_n, universe_epoch, at_epoch)
            .await?;
    let t_s =
        resolve_advective_surface_temperature(pool, planet_id, lat_s, universe_epoch, at_epoch)
            .await?;
    let t_local =
        resolve_advective_surface_temperature(pool, planet_id, latitude, universe_epoch, at_epoch)
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

pub async fn resolve_stellar_wind_at_planet(
    pool: &SqlitePool,
    planet_id: Uuid,
    eta: f64,
    wind_scaling: f64,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<StellarWindDiagnostic> {
    let planet_row = planet_repository::get_by_id(pool, &planet_id)
        .await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, &planet).await?;

    let star_temp = star
        .effective_temperature()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "effective_temperature".to_string(),
            reason: "star does not have effective temperature".to_string(),
        })?;

    let star_radius = star
        .radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: "star does not have radius".to_string(),
        })?;

    let system_id = star
        .star_system_id()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_system_id".to_string(),
            reason: "parent star is not assigned to a star system".to_string(),
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let positions = resolve_system_positions(pool, system_id, total_epoch).await?;

    let planet_pos = positions
        .get(&planet.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("position for planet '{}' could not be resolved", planet.id()),
        })?;

    let star_pos = positions
        .get(&star.id())
        .copied()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "star_id".to_string(),
            reason: format!("position for star '{}' could not be resolved", star.id()),
        })?;

    let orbital_distance = (planet_pos - star_pos).magnitude();
    let star_lum = stellar_luminosity(star_radius, star_temp);
    let mu_star = gravitational_parameter(star.mass());
    let v_esc = escape_velocity(mu_star, star_radius);

    let m_dot = reimers_mass_loss_rate(star_lum, star_radius, star.mass(), eta);
    let v_inf = terminal_wind_speed(v_esc, wind_scaling);
    let rho_sw = stellar_wind_density(m_dot, v_inf, orbital_distance);
    let p_dyn = stellar_wind_dynamic_pressure(rho_sw, v_inf);

    Ok(StellarWindDiagnostic {
        mass_loss_rate: m_dot,
        escape_velocity: v_esc,
        terminal_wind_speed: v_inf,
        wind_density_at_orbit: rho_sw,
        dynamic_pressure: p_dyn,
    })
}

pub async fn resolve_atmospheric_profile_at_altitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    surface_temperature: Temperature,
    altitude: Length,
) -> AppResult<(Pressure, Temperature, Density)> {
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

    let eq_radius = planet
        .equatorial_radius()
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "equatorial_radius".to_string(),
            reason: "planet does not have equatorial radius".to_string(),
        })?;

    let mu = gravitational_parameter(planet.mass());
    let gravity = surface_gravity(mu, eq_radius);

    let temp_at_alt =
        temperature_at_altitude(surface_temperature, altitude, atmosphere.lapse_rate());
    let scale_h = atmosphere.scale_height(gravity, surface_temperature)?;
    let press_at_alt = atmosphere.pressure_at_altitude(altitude, scale_h);
    let molar_mass = atmosphere.mean_molar_mass()?;
    let density_at_alt = ideal_gas_density(press_at_alt, molar_mass, temp_at_alt);

    Ok((press_at_alt, temp_at_alt, density_at_alt))
}