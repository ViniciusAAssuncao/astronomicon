use crate::black_hole::{ resolve_black_hole_accretion, resolve_black_hole_diagnostics };
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use astronomicon_core::domain::{
    Barycenter,
    BarycenterMember,
    MinorPlanet,
    OrbitalParent,
    Planet,
    Star,
    StarKind,
};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::black_hole::gravitational_redshift_between;
use astronomicon_core::math::circulation::{
    circulation_cells_per_hemisphere,
    equatorial_rossby_deformation_radius,
    rhines_scale,
};
use astronomicon_core::math::climate::{
    advective_local_temperature,
    atmospheric_column_heat_capacity,
    blended_local_temperature,
    combined_column_heat_capacity,
    combined_thermal_redistribution_efficiency,
    day_length_half_angle,
    local_radiative_equilibrium_temperature,
    mean_daily_insolation_factor,
    solar_declination,
    temperature_at_altitude,
    thermal_redistribution_efficiency,
};
use astronomicon_core::math::gravity::{
    combined_gravitational_parameter,
    gravitational_parameter,
    surface_gravity,
};
use astronomicon_core::math::kepler::true_anomaly_at_epoch;
use astronomicon_core::math::radiometry::{
    escape_velocity,
    orbital_irradiance,
    stellar_luminosity,
};
use astronomicon_core::math::rotation::{
    angular_velocity_from_rotation_period,
    coriolis_parameter,
    rossby_beta_parameter,
};
use astronomicon_core::math::stellar_wind::{
    reimers_mass_loss_rate,
    stellar_wind_density,
    stellar_wind_dynamic_pressure,
    terminal_wind_speed,
};
use astronomicon_core::math::thermodynamics::{
    cloud_top_altitude,
    dew_point_temperature,
    lifting_condensation_level,
    moist_adiabatic_lapse_rate,
};
use astronomicon_core::math::wind::{
    latitudinal_temperature_gradient,
    surface_wind_components,
    surface_wind_speed,
    zonal_jet_stream_speed,
};
use astronomicon_core::units::constants::STEFAN_BOLTZMANN_CONSTANT;
use astronomicon_core::units::{
    Angle,
    AngularVelocity,
    Density,
    Duration,
    Irradiance,
    Length,
    Luminosity,
    Mass,
    MassRate,
    MolarMass,
    Pressure,
    Speed,
    Temperature,
    TemperatureGradient,
};
use astronomicon_db::repositories::{
    atmosphere_repository,
    barycenter_repository,
    hydrosphere_repository,
    minor_planet_repository,
    planet_repository,
    star_repository,
};
use astronomicon_db::SqlitePool;
use serde::{ Deserialize, Serialize };
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericStratificationDiagnostic {
    pub surface_dew_point: Temperature,
    pub lcl_altitude: Length,
    pub cloud_top_altitude: Length,
    pub moist_adiabatic_lapse_rate: TemperatureGradient,
}

pub fn resolve_stellar_wind_at_distance(
    star_mass: Mass,
    star_radius: Length,
    star_temp: Temperature,
    orbital_distance: Length,
    eta: f64,
    wind_scaling: f64
) -> StellarWindDiagnostic {
    let star_lum = stellar_luminosity(star_radius, star_temp);
    let mu_star = gravitational_parameter(star_mass);
    let v_esc = escape_velocity(mu_star, star_radius);

    let m_dot = reimers_mass_loss_rate(star_lum, star_radius, star_mass, eta);
    let v_inf = terminal_wind_speed(v_esc, wind_scaling);
    let rho_sw = stellar_wind_density(m_dot, v_inf, orbital_distance);
    let p_dyn = stellar_wind_dynamic_pressure(rho_sw, v_inf);

    StellarWindDiagnostic {
        mass_loss_rate: m_dot,
        escape_velocity: v_esc,
        terminal_wind_speed: v_inf,
        wind_density_at_orbit: rho_sw,
        dynamic_pressure: p_dyn,
    }
}

pub async fn resolve_star_emission_profile(
    pool: &SqlitePool,
    star: &Star,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<(Luminosity, Temperature, Length)> {
    if star.kind() == StarKind::BlackHole {
        let acc = resolve_black_hole_accretion(
            pool,
            star.id(),
            1.0,
            1.0,
            universe_epoch,
            at_epoch
        ).await?;
        let bh_diag = resolve_black_hole_diagnostics(pool, star.id()).await?;
        let r_emit = bh_diag.isco_radius_prograde;

        let eff_temp = if acc.accretion_luminosity.value() > 0.0 {
            let area = 4.0 * PI * r_emit.value() * r_emit.value();
            if area > 0.0 {
                let t4 = acc.accretion_luminosity.value() / (area * STEFAN_BOLTZMANN_CONSTANT);
                Temperature::new(t4.max(0.0).powf(0.25))
            } else {
                bh_diag.hawking_temperature
            }
        } else {
            bh_diag.hawking_temperature
        };

        Ok((acc.total_luminosity, eff_temp, r_emit))
    } else {
        let star_temp = star.effective_temperature().ok_or_else(|| DomainError::InvalidInvariant {
            field: "effective_temperature".to_string(),
            reason: "star does not have effective temperature".to_string(),
        })?;
        let star_radius = star.radius().ok_or_else(|| DomainError::InvalidInvariant {
            field: "radius".to_string(),
            reason: "star does not have radius".to_string(),
        })?;
        let star_lum = stellar_luminosity(star_radius, star_temp);
        Ok((star_lum, star_temp, star_radius))
    }
}

pub(crate) async fn collect_stars_from_barycenter(
    pool: &SqlitePool,
    barycenter_id: &Uuid,
    visited: &mut std::collections::HashSet<Uuid>
) -> AppResult<Vec<Star>> {
    if !visited.insert(*barycenter_id) {
        return Err(
            (DomainError::InvalidInvariant {
                field: "barycenter".to_string(),
                reason: format!("circular reference detected in barycenter '{}'", barycenter_id),
            }).into()
        );
    }

    let row = barycenter_repository
        ::get_by_id(pool, barycenter_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "barycenter_id".to_string(),
            reason: format!("barycenter '{}' not found", barycenter_id),
        })?;
    let barycenter = Barycenter::try_from(row)?;

    let mut stars = Vec::new();

    for member in [barycenter.member_primary(), barycenter.member_secondary()] {
        match member {
            BarycenterMember::Star(star_id) => {
                let star_row = star_repository
                    ::get_by_id(pool, &star_id).await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "star_id".to_string(),
                        reason: format!("star '{}' in barycenter not found", star_id),
                    })?;
                stars.push(Star::try_from(star_row)?);
            }
            BarycenterMember::Planet(_) => {}
            BarycenterMember::Barycenter(sub_id) => {
                let mut sub_stars = Box::pin(
                    collect_stars_from_barycenter(pool, &sub_id, visited)
                ).await?;
                stars.append(&mut sub_stars);
            }
        }
    }

    visited.remove(barycenter_id);
    Ok(stars)
}

pub(crate) async fn find_parent_star(pool: &SqlitePool, planet: &Planet) -> AppResult<Star> {
    let mut current_parent = planet.orbital_parent();
    let mut visited_barycenters = std::collections::HashSet::new();

    loop {
        match current_parent {
            OrbitalParent::Star(star_id) => {
                let row = star_repository
                    ::get_by_id(pool, &star_id).await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_star_id".to_string(),
                        reason: format!("parent star '{}' not found", star_id),
                    })?;
                return Ok(Star::try_from(row)?);
            }
            OrbitalParent::Planet(planet_id) => {
                let row = planet_repository
                    ::get_by_id(pool, &planet_id).await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_planet_id".to_string(),
                        reason: format!("parent planet '{}' not found", planet_id),
                    })?;
                let parent_planet = Planet::try_from(row)?;
                current_parent = parent_planet.orbital_parent();
            }
            OrbitalParent::MinorPlanet(mp_id) => {
                let row = minor_planet_repository
                    ::get_by_id(pool, &mp_id).await?
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "parent_minor_planet_id".to_string(),
                        reason: format!("parent minor planet '{}' not found", mp_id),
                    })?;
                let parent_mp = MinorPlanet::try_from(row)?;
                current_parent = parent_mp.orbital_parent();
            }
            OrbitalParent::Barycenter(barycenter_id) => {
                let stars = collect_stars_from_barycenter(
                    pool,
                    &barycenter_id,
                    &mut visited_barycenters
                ).await?;
                let most_massive = stars
                    .into_iter()
                    .max_by(|a, b| {
                        a.mass().partial_cmp(&b.mass()).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .ok_or_else(|| DomainError::InvalidInvariant {
                        field: "barycenter_stars".to_string(),
                        reason: format!("no stars found in barycenter '{}' hierarchy", barycenter_id),
                    })?;
                return Ok(most_massive);
            }
            OrbitalParent::Fixed => {
                return Err(
                    (DomainError::InvalidInvariant {
                        field: "planet_hierarchy".to_string(),
                        reason: "planet has no parent star in hierarchy".to_string(),
                    }).into()
                );
            }
        }
    }
}

pub async fn resolve_global_mean_temperature(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<Temperature> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let bond_albedo = planet.bond_albedo().unwrap_or(0.3);

    let star = find_parent_star(pool, &planet).await?;
    let (star_lum, _, r_emit) = resolve_star_emission_profile(
        pool,
        &star,
        universe_epoch,
        at_epoch
    ).await?;

    let system_id = star.star_system_id().ok_or_else(|| DomainError::InvalidInvariant {
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
    let z_factor = if star.kind() == StarKind::BlackHole {
        gravitational_redshift_between(star.mass(), r_emit, orbital_distance)
    } else {
        1.0
    };

    let base_irradiance = orbital_irradiance(star_lum, orbital_distance);
    let top_irradiance = Irradiance::new(base_irradiance.value() / (z_factor * z_factor));

    let greenhouse = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atmosphere) => atmosphere.greenhouse_effect(),
        None => Temperature::new(0.0),
    };

    let effective_albedo = if
        let Some(hydrosphere) = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?
    {
        let base_eq = local_radiative_equilibrium_temperature(
            Irradiance::new(top_irradiance.value() * 0.25),
            bond_albedo
        );
        let base_surface_temp = base_eq + greenhouse;
        let pressure = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
            Some(atm) => atm.surface_pressure(),
            None => Pressure::new(0.0),
        };
        let initial_state = hydrosphere.matter_state(base_surface_temp, pressure)?;
        hydrosphere.dynamic_albedo(bond_albedo, initial_state)?
    } else {
        bond_albedo
    };

    let eq_temp = local_radiative_equilibrium_temperature(
        Irradiance::new(top_irradiance.value() * 0.25),
        effective_albedo
    );

    Ok(eq_temp + greenhouse)
}

pub async fn resolve_latitudinal_surface_temperature(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<Temperature> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, &planet).await?;

    let thermal_inertia = planet.thermal_inertia().unwrap_or(0.0);
    let obliquity = planet.obliquity().unwrap_or_else(|| Angle::new(0.0));
    let solstice_true_anomaly = planet.solstice_true_anomaly().unwrap_or_else(|| Angle::new(0.0));

    let orbital_elements = planet.orbital_elements().ok_or_else(|| DomainError::InvalidInvariant {
        field: "orbital_elements".to_string(),
        reason: "planet does not have orbital elements".to_string(),
    })?;

    let bond_albedo = planet.bond_albedo().unwrap_or(0.3);

    let (star_lum, _, r_emit) = resolve_star_emission_profile(
        pool,
        &star,
        universe_epoch,
        at_epoch
    ).await?;

    let system_id = star.star_system_id().ok_or_else(|| DomainError::InvalidInvariant {
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
        true_anomaly
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
    let z_factor = if star.kind() == StarKind::BlackHole {
        gravitational_redshift_between(star.mass(), r_emit, orbital_distance)
    } else {
        1.0
    };

    let base_irradiance = orbital_irradiance(star_lum, orbital_distance);
    let top_irradiance = Irradiance::new(base_irradiance.value() / (z_factor * z_factor));
    let local_insolation = top_irradiance * insolation_factor;

    let local_eq = local_radiative_equilibrium_temperature(local_insolation, bond_albedo);
    let global_mean = resolve_global_mean_temperature(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let blended = blended_local_temperature(global_mean, local_eq, thermal_inertia);

    Ok(blended)
}

pub async fn resolve_advective_surface_temperature(
    pool: &SqlitePool,
    planet_id: Uuid,
    latitude: Angle,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<Temperature> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, &planet).await?;

    let obliquity = planet.obliquity().unwrap_or_else(|| Angle::new(0.0));
    let solstice_true_anomaly = planet.solstice_true_anomaly().unwrap_or_else(|| Angle::new(0.0));

    let orbital_elements = planet.orbital_elements().ok_or_else(|| DomainError::InvalidInvariant {
        field: "orbital_elements".to_string(),
        reason: "planet does not have orbital elements".to_string(),
    })?;

    let bond_albedo = planet.bond_albedo().unwrap_or(0.3);

    let (star_lum, _, r_emit) = resolve_star_emission_profile(
        pool,
        &star,
        universe_epoch,
        at_epoch
    ).await?;

    let system_id = star.star_system_id().ok_or_else(|| DomainError::InvalidInvariant {
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
        true_anomaly
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
    let z_factor = if star.kind() == StarKind::BlackHole {
        gravitational_redshift_between(star.mass(), r_emit, orbital_distance)
    } else {
        1.0
    };

    let base_irradiance = orbital_irradiance(star_lum, orbital_distance);
    let top_irradiance = Irradiance::new(base_irradiance.value() / (z_factor * z_factor));
    let local_insolation = top_irradiance * insolation_factor;

    let local_eq = local_radiative_equilibrium_temperature(local_insolation, bond_albedo);
    let global_mean = resolve_global_mean_temperature(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;

    let circulation = resolve_planetary_circulation(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let advective = advective_local_temperature(
        global_mean,
        local_eq,
        circulation.thermal_redistribution_efficiency
    );

    Ok(advective)
}

pub async fn resolve_planetary_circulation(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<PlanetaryCirculationDiagnostic> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let radius = planet.equatorial_radius().ok_or_else(|| DomainError::InvalidInvariant {
        field: "equatorial_radius".to_string(),
        reason: "planet does not have equatorial radius".to_string(),
    })?;

    let rot_period = planet.rotation_period().unwrap_or_else(|| Duration::new(86400.0));
    let omega = angular_velocity_from_rotation_period(rot_period);
    let beta_eq = rossby_beta_parameter(omega, Angle::new(0.0), radius);

    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, radius);

    let global_mean = resolve_global_mean_temperature(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;

    let (scale_h, atm_col_heat_cap) = if
        let Some(atmosphere) = atmosphere_repository::get_by_planet_id(pool, &planet_id).await?
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
        at_epoch
    ).await?;
    let temp_pole = resolve_latitudinal_surface_temperature(
        pool,
        planet_id,
        Angle::new(PI / 2.0),
        universe_epoch,
        at_epoch
    ).await?;

    let delta_t = (temp_eq.value() - temp_pole.value()).abs().max(1.0);
    let char_u = Speed::new((g.value() * scale_h.value() * (delta_t / global_mean.value())).sqrt());
    let l_beta = rhines_scale(char_u, beta_eq);

    let cells = circulation_cells_per_hemisphere(radius, l_beta);

    let (efficiency, col_heat_cap) = if
        let Some(hydrosphere) = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?
    {
        let oce_col_heat_cap = hydrosphere.oceanic_column_heat_capacity()?;
        let cov = hydrosphere.surface_coverage_fraction();
        let eff = combined_thermal_redistribution_efficiency(
            atm_col_heat_cap,
            oce_col_heat_cap,
            cov,
            cells
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
    at_epoch: Duration
) -> AppResult<WindProfileDiagnostic> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let radius = planet.equatorial_radius().ok_or_else(|| DomainError::InvalidInvariant {
        field: "equatorial_radius".to_string(),
        reason: "planet does not have equatorial radius".to_string(),
    })?;

    let rot_period = planet.rotation_period().unwrap_or_else(|| Duration::new(86400.0));
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
        at_epoch
    ).await?;
    let t_s = resolve_advective_surface_temperature(
        pool,
        planet_id,
        lat_s,
        universe_epoch,
        at_epoch
    ).await?;
    let t_local = resolve_advective_surface_temperature(
        pool,
        planet_id,
        latitude,
        universe_epoch,
        at_epoch
    ).await?;

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
    at_epoch: Duration
) -> AppResult<StellarWindDiagnostic> {
    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = find_parent_star(pool, &planet).await?;

    if star.kind() == StarKind::BlackHole {
        return Ok(StellarWindDiagnostic {
            mass_loss_rate: MassRate::new(0.0),
            escape_velocity: Speed::new(0.0),
            terminal_wind_speed: Speed::new(0.0),
            wind_density_at_orbit: Density::new(0.0),
            dynamic_pressure: Pressure::new(0.0),
        });
    }

    let star_temp = star.effective_temperature().ok_or_else(|| DomainError::InvalidInvariant {
        field: "effective_temperature".to_string(),
        reason: "star does not have effective temperature".to_string(),
    })?;

    let star_radius = star.radius().ok_or_else(|| DomainError::InvalidInvariant {
        field: "radius".to_string(),
        reason: "star does not have radius".to_string(),
    })?;

    let system_id = star.star_system_id().ok_or_else(|| DomainError::InvalidInvariant {
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

    Ok(
        resolve_stellar_wind_at_distance(
            star.mass(),
            star_radius,
            star_temp,
            orbital_distance,
            eta,
            wind_scaling
        )
    )
}

pub async fn resolve_atmospheric_profile_at_altitude(
    pool: &SqlitePool,
    planet_id: Uuid,
    surface_temperature: Temperature,
    altitude: Length
) -> AppResult<(Pressure, Temperature, Density)> {
    let atmosphere = atmosphere_repository
        ::get_by_planet_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let eq_radius = planet.equatorial_radius().ok_or_else(|| DomainError::InvalidInvariant {
        field: "equatorial_radius".to_string(),
        reason: "planet does not have equatorial radius".to_string(),
    })?;

    let mu = gravitational_parameter(planet.mass());
    let gravity = surface_gravity(mu, eq_radius);

    let temp_at_alt = temperature_at_altitude(
        surface_temperature,
        altitude,
        atmosphere.lapse_rate()
    );
    let scale_h = atmosphere.scale_height(gravity, surface_temperature)?;
    let press_at_alt = atmosphere.pressure_at_altitude(altitude, scale_h);
    let molar_mass = atmosphere.mean_molar_mass()?;
    let density_at_alt = ideal_gas_density(press_at_alt, molar_mass, temp_at_alt);

    Ok((press_at_alt, temp_at_alt, density_at_alt))
}

pub async fn resolve_atmospheric_stratification(
    pool: &SqlitePool,
    planet_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration
) -> AppResult<AtmosphericStratificationDiagnostic> {
    let atmosphere = atmosphere_repository
        ::get_by_planet_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "atmosphere".to_string(),
            reason: format!("planet '{}' has no atmosphere", planet_id),
        })?;

    let planet_row = planet_repository
        ::get_by_id(pool, &planet_id).await?
        .ok_or_else(|| DomainError::InvalidInvariant {
            field: "planet_id".to_string(),
            reason: format!("planet '{}' not found", planet_id),
        })?;
    let planet = Planet::try_from(planet_row)?;

    let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let mu = gravitational_parameter(planet.mass());
    let g = surface_gravity(mu, eq_radius);

    let surf_temp = resolve_global_mean_temperature(
        pool,
        planet_id,
        universe_epoch,
        at_epoch
    ).await?;
    let surf_press = atmosphere.surface_pressure();
    let scale_h = atmosphere.scale_height(g, surf_temp)?;
    let atm_molar_mass = atmosphere.mean_molar_mass()?;
    let atm_cp = atmosphere.mean_specific_heat_capacity()?;
    let env_lapse_rate = atmosphere.lapse_rate();

    let hydro_opt = hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?;

    let (solvent_props, solvent_molar_mass, humidity) = if let Some(hydro) = hydro_opt {
        let props = hydro.mean_solvent_properties()?;
        let mapped: Vec<(String, f64)> = hydro
            .composition()
            .iter()
            .map(|c| (c.formula().to_string(), c.percentage()))
            .collect();
        let mm = astronomicon_core::chemistry
            ::mean_molar_mass(&mapped)
            .unwrap_or_else(|_| MolarMass::new(0.018015));
        let hum = atmosphere
            .surface_humidity()
            .unwrap_or(0.6 * hydro.surface_coverage_fraction().clamp(0.1, 1.0));
        (props, mm, hum)
    } else {
        let found = atmosphere
            .composition()
            .iter()
            .find_map(|c| {
                let formula = c.formula();
                astronomicon_core::chemistry::solvent_properties_of(formula).and_then(|p| {
                    astronomicon_core::chemistry
                        ::molar_mass_of(formula)
                        .ok()
                        .map(|mm| (p, mm))
                })
            });

        let (props, mm) = match found {
            Some((p, mm)) => (p, mm),
            None => {
                let default_p = astronomicon_core::chemistry
                    ::solvent_properties_of("H2O")
                    .expect("H2O solvent properties");
                let default_mm = MolarMass::new(0.018015);
                (default_p, default_mm)
            }
        };
        let hum = atmosphere.surface_humidity().unwrap_or(0.0);
        (props, mm, hum)
    };

    let dew_point = dew_point_temperature(
        surf_temp,
        humidity,
        solvent_props.enthalpy_of_vaporization
    );
    let moist_gamma = moist_adiabatic_lapse_rate(
        g,
        atm_cp,
        surf_temp,
        surf_press,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass
    );

    let dry_gamma = if env_lapse_rate.value() > 0.0 {
        env_lapse_rate
    } else {
        TemperatureGradient::new(g.value() / atm_cp.max(100.0))
    };

    let lcl = lifting_condensation_level(
        surf_temp,
        dew_point,
        dry_gamma,
        scale_h,
        solvent_props.enthalpy_of_vaporization
    );

    let cloud_top = cloud_top_altitude(
        lcl,
        surf_temp,
        surf_press,
        dry_gamma,
        moist_gamma,
        scale_h,
        g,
        &solvent_props,
        solvent_molar_mass,
        atm_molar_mass
    );

    Ok(AtmosphericStratificationDiagnostic {
        surface_dew_point: dew_point,
        lcl_altitude: lcl,
        cloud_top_altitude: cloud_top,
        moist_adiabatic_lapse_rate: moist_gamma,
    })
}
