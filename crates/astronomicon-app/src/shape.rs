use crate::error::AppResult;
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::error::DomainError;
use astronomicon_core::math::shape::{
    oblate_spheroid_mean_density, polar_radius_from_flattening, rotational_flattening,
};
use astronomicon_core::units::{Density, Length};
use astronomicon_db::repositories::{planet_repository, star_repository};
use astronomicon_db::SqlitePool;
use uuid::Uuid;

pub fn effective_polar_radius_for_planet(planet: &Planet) -> Length {
    if let Some(polar_r) = planet.polar_radius() {
        return polar_r;
    }

    let eq_r = match planet.equatorial_radius() {
        Some(r) => r,
        None => return Length::new(0.0),
    };

    if let (Some(j2), Some(rot_period)) = (planet.oblateness_j2(), planet.rotation_period()) {
        let f = rotational_flattening(planet.mass(), eq_r, rot_period, j2);
        polar_radius_from_flattening(eq_r, f)
    } else {
        eq_r
    }
}

pub fn effective_polar_radius_for_star(star: &Star) -> Length {
    let eq_r = match star.radius() {
        Some(r) => r,
        None => return Length::new(0.0),
    };

    if let (Some(j2), Some(rot_period)) = (star.oblateness_j2(), star.rotation_period()) {
        let f = rotational_flattening(star.mass(), eq_r, rot_period, j2);
        polar_radius_from_flattening(eq_r, f)
    } else {
        eq_r
    }
}

pub fn planet_mean_density(planet: &Planet) -> Density {
    let eq_r = planet.equatorial_radius().unwrap_or(Length::new(0.0));
    let pol_r = effective_polar_radius_for_planet(planet);
    oblate_spheroid_mean_density(planet.mass(), eq_r, pol_r)
}

pub fn star_mean_density(star: &Star) -> Density {
    let eq_r = star.radius().unwrap_or(Length::new(0.0));
    let pol_r = effective_polar_radius_for_star(star);
    oblate_spheroid_mean_density(star.mass(), eq_r, pol_r)
}

pub async fn resolve_mean_density(pool: &SqlitePool, entity_id: Uuid) -> AppResult<Density> {
    if let Some(planet_row) = planet_repository::get_by_id(pool, &entity_id).await? {
        let planet = Planet::try_from(planet_row)?;
        return Ok(planet_mean_density(&planet));
    }

    if let Some(star_row) = star_repository::get_by_id(pool, &entity_id).await? {
        let star = Star::try_from(star_row)?;
        return Ok(star_mean_density(&star));
    }

    Err(DomainError::InvalidInvariant {
        field: "entity_id".to_string(),
        reason: format!("entity '{}' not found", entity_id),
    }
    .into())
}