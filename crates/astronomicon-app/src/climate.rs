use crate::ephemeris::resolve_system_positions;
use crate::error::AppResult;
use astronomicon_core::domain::{Barycenter, BarycenterMember, OrbitalParent, Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::climate::{
    blended_local_temperature, day_length_half_angle, local_radiative_equilibrium_temperature,
    mean_daily_insolation_factor, solar_declination, temperature_at_altitude,
};
use astronomicon_core::math::gravity::{
    combined_gravitational_parameter, gravitational_parameter, surface_gravity,
};
use astronomicon_core::math::kepler::true_anomaly_at_epoch;
use astronomicon_core::math::radiometry::{
    equilibrium_temperature, orbital_irradiance, stellar_luminosity,
};
use astronomicon_core::units::{Angle, Density, Duration, Length, Pressure, Temperature};
use astronomicon_db::repositories::{
    atmosphere_repository, barycenter_repository, planet_repository, star_repository,
};
use astronomicon_db::SqlitePool;
use uuid::Uuid;

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