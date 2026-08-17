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

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::domain::{OrbitalParent, PlanetKind, StarKind};
    use astronomicon_core::units::{Duration, Mass};

    #[test]
    fn test_planet_effective_polar_radius_priority() {
        let id = Uuid::new_v4();
        let m = Mass::new(5.9722e24);
        let r_eq = Length::new(6.378137e6);
        let r_pol = Length::new(6.356752e6);

        let p_explicit = Planet::builder(id, "Terra", m, PlanetKind::Telluric, OrbitalParent::Fixed)
            .with_equatorial_radius(r_eq)
            .with_polar_radius(r_pol)
            .with_rotation_period(Duration::new(86164.0905))
            .with_oblateness_j2(0.00108263)
            .build()
            .unwrap();
        assert_eq!(effective_polar_radius_for_planet(&p_explicit), r_pol);

        let p_derived = Planet::builder(id, "Terra", m, PlanetKind::Telluric, OrbitalParent::Fixed)
            .with_equatorial_radius(r_eq)
            .with_rotation_period(Duration::new(86164.0905))
            .with_oblateness_j2(0.00108263)
            .build()
            .unwrap();
        let r_derived = effective_polar_radius_for_planet(&p_derived);
        assert!((r_derived.value() - 6.35675e6).abs() < 100.0);

        let p_spherical = Planet::builder(id, "Terra", m, PlanetKind::Telluric, OrbitalParent::Fixed)
            .with_equatorial_radius(r_eq)
            .build()
            .unwrap();
        assert_eq!(effective_polar_radius_for_planet(&p_spherical), r_eq);
    }

    #[test]
    fn test_star_effective_polar_radius_priority() {
        let id = Uuid::new_v4();
        let m = Mass::new(1.98847e30);
        let r_eq = Length::new(6.957e8);

        let s_derived = Star::builder(id, "Sol", m, StarKind::Star, OrbitalParent::Fixed)
            .with_radius(r_eq)
            .with_rotation_period(Duration::new(2.1e6))
            .with_oblateness_j2(0.00002)
            .build()
            .unwrap();
        let r_derived = effective_polar_radius_for_star(&s_derived);
        assert!(r_derived.value() < r_eq.value());

        let s_spherical = Star::builder(id, "Sol", m, StarKind::Star, OrbitalParent::Fixed)
            .with_radius(r_eq)
            .build()
            .unwrap();
        assert_eq!(effective_polar_radius_for_star(&s_spherical), r_eq);
    }
}