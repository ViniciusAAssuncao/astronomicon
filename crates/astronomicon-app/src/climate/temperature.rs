use crate::climate::circulation::resolve_planetary_circulation;
use crate::climate::emission::resolve_star_emission_profile;
use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use crate::hierarchy::find_parent_star;
use astronomicon_core::domain::{Planet, Star, StarKind};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::black_hole::gravitational_redshift_between;
use astronomicon_core::math::climate::{
    advective_local_temperature, blended_local_temperature, day_length_half_angle,
    local_radiative_equilibrium_temperature, mean_daily_insolation_factor, solar_declination,
};
use astronomicon_core::math::gravity::combined_gravitational_parameter;
use astronomicon_core::math::kepler::true_anomaly_at_epoch;
use astronomicon_core::math::radiometry::orbital_irradiance;
use astronomicon_core::units::{Angle, Duration, Irradiance, Pressure, Temperature};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{
    atmosphere_repository, hydrosphere_repository, planet_repository,
};
use uuid::Uuid;

pub async fn resolve_top_of_atmosphere_irradiance(
    pool: &SqlitePool,
    planet: &Planet,
    star: &Star,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> AppResult<Irradiance> {
    let (star_lum, _, r_emit) =
        resolve_star_emission_profile(pool, star, universe_epoch, at_epoch).await?;

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
    Ok(Irradiance::new(base_irradiance.value() / (z_factor * z_factor)))
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

    let bond_albedo = planet.bond_albedo().unwrap_or(0.3);
    let star = find_parent_star(pool, planet.orbital_parent()).await?;
    let top_irradiance =
        resolve_top_of_atmosphere_irradiance(pool, &planet, &star, universe_epoch, at_epoch)
            .await?;

    let greenhouse = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atmosphere) => atmosphere.greenhouse_effect(),
        None => Temperature::new(0.0),
    };

    let effective_albedo = if let Some(hydrosphere) =
        hydrosphere_repository::get_by_planet_id(pool, &planet_id).await?
    {
        let base_eq = local_radiative_equilibrium_temperature(
            Irradiance::new(top_irradiance.value() * 0.25),
            bond_albedo,
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
        effective_albedo,
    );

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

    let star = find_parent_star(pool, planet.orbital_parent()).await?;

    let thermal_inertia = planet.thermal_inertia().unwrap_or(0.0);
    let obliquity = planet.obliquity().unwrap_or_else(|| Angle::new(0.0));
    let solstice_true_anomaly = planet
        .solstice_true_anomaly()
        .unwrap_or_else(|| Angle::new(0.0));

    let orbital_elements =
        planet
            .orbital_elements()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "planet does not have orbital elements".to_string(),
            })?;

    let bond_albedo = planet.bond_albedo().unwrap_or(0.3);
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

    let top_irradiance =
        resolve_top_of_atmosphere_irradiance(pool, &planet, &star, universe_epoch, at_epoch)
            .await?;
    let local_insolation = top_irradiance * insolation_factor;

    let greenhouse = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atmosphere) => atmosphere.greenhouse_effect(),
        None => Temperature::new(0.0),
    };

    let local_eq = local_radiative_equilibrium_temperature(local_insolation, bond_albedo);
    let local_surface_temp = local_eq + greenhouse;
    let global_mean =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;
    let blended = blended_local_temperature(global_mean, local_surface_temp, thermal_inertia);

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

    let star = find_parent_star(pool, planet.orbital_parent()).await?;

    let obliquity = planet.obliquity().unwrap_or_else(|| Angle::new(0.0));
    let solstice_true_anomaly = planet
        .solstice_true_anomaly()
        .unwrap_or_else(|| Angle::new(0.0));

    let orbital_elements =
        planet
            .orbital_elements()
            .ok_or_else(|| DomainError::InvalidInvariant {
                field: "orbital_elements".to_string(),
                reason: "planet does not have orbital elements".to_string(),
            })?;

    let bond_albedo = planet.bond_albedo().unwrap_or(0.3);
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

    let top_irradiance =
        resolve_top_of_atmosphere_irradiance(pool, &planet, &star, universe_epoch, at_epoch)
            .await?;
    let local_insolation = top_irradiance * insolation_factor;

    let greenhouse = match atmosphere_repository::get_by_planet_id(pool, &planet_id).await? {
        Some(atmosphere) => atmosphere.greenhouse_effect(),
        None => Temperature::new(0.0),
    };

    let local_eq = local_radiative_equilibrium_temperature(local_insolation, bond_albedo);
    let local_surface_temp = local_eq + greenhouse;
    let global_mean =
        resolve_global_mean_temperature(pool, planet_id, universe_epoch, at_epoch).await?;

    let circulation =
        resolve_planetary_circulation(pool, planet_id, universe_epoch, at_epoch).await?;
    let advective = advective_local_temperature(
        global_mean,
        local_surface_temp,
        circulation.thermal_redistribution_efficiency,
    );

    Ok(advective)
}