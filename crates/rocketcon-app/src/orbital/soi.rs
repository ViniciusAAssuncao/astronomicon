use crate::error::{RocketError, RocketResult};
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::units::{Duration, Length, Position, Temperature, VelocityVector};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository, star_repository};
use rocketcon_core::math::orbital::sphere_of_influence::{
    laplace_sphere_of_influence_radius, CelestialBodySoi,
};
use uuid::Uuid;

pub async fn resolve_body_state_at_epoch(
    pool: &SqlitePool,
    body_id: Uuid,
    system_id: Uuid,
    total_epoch: Duration,
) -> RocketResult<(Position, VelocityVector)> {
    let eps = 0.1;
    let pos1 = astronomicon_app::ephemeris::resolve_system_positions(pool, system_id, total_epoch).await?;
    let pos2 = astronomicon_app::ephemeris::resolve_system_positions(pool, system_id, total_epoch + Duration::new(eps)).await?;
    let p1 = pos1.get(&body_id).copied().ok_or_else(|| {
        RocketError::Generic(format!("position for body '{}' not found in system '{}'", body_id, system_id))
    })?;
    let p2 = pos2.get(&body_id).copied().unwrap_or(p1);
    let vel = VelocityVector::from_raw((p2.raw() - p1.raw()) / eps);
    Ok((p1, vel))
}

pub async fn resolve_system_soi_bodies(
    pool: &SqlitePool,
    system_id: Uuid,
    total_epoch: Duration,
) -> RocketResult<Vec<CelestialBodySoi>> {
    let stars_rows = star_repository::list_by_system(pool, &system_id).await?;
    let mut stars = Vec::with_capacity(stars_rows.len());
    for row in stars_rows {
        stars.push(Star::try_from(row)?);
    }

    let planets_rows = planet_repository::list_by_system(pool, &system_id).await?;
    let mut planets = Vec::with_capacity(planets_rows.len());
    for row in planets_rows {
        planets.push(Planet::try_from(row)?);
    }

    let positions = astronomicon_app::ephemeris::resolve_system_positions(pool, system_id, total_epoch).await?;

    let mut result = Vec::new();

    for star in &stars {
        let star_pos = positions.get(&star.id()).copied().unwrap_or_else(Position::zero);
        result.push(CelestialBodySoi::new_with_geometry(
            star.id(),
            None,
            star_pos,
            star.mass(),
            Length::new(f64::INFINITY),
            star.radius().unwrap_or_else(|| Length::new(6.957e8)),
        ));
    }

    for planet in &planets {
        let planet_pos = positions.get(&planet.id()).copied().unwrap_or_else(Position::zero);

        let (parent_id, parent_mass, sma) = match planet.orbital_parent() {
            astronomicon_core::domain::OrbitalParent::Star(star_id) => {
                let parent_star = stars.iter().find(|s| s.id() == star_id);
                let mass = parent_star.map(|s| s.mass()).unwrap_or_else(|| astronomicon_core::units::Mass::new(1.989e30));
                let sma = planet.orbital_elements().map(|e| e.semi_major_axis()).unwrap_or_else(|| Length::new(1.496e11));
                (Some(star_id), mass, sma)
            }
            astronomicon_core::domain::OrbitalParent::Planet(parent_planet_id) => {
                let parent_p = planets.iter().find(|p| p.id() == parent_planet_id);
                let mass = parent_p.map(|p| p.mass()).unwrap_or_else(|| astronomicon_core::units::Mass::new(5.972e24));
                let sma = planet.orbital_elements().map(|e| e.semi_major_axis()).unwrap_or_else(|| Length::new(3.844e8));
                (Some(parent_planet_id), mass, sma)
            }
            _ => {
                let default_mass = stars.first().map(|s| s.mass()).unwrap_or_else(|| astronomicon_core::units::Mass::new(1.989e30));
                let default_star_id = stars.first().map(|s| s.id());
                let sma = planet.orbital_elements().map(|e| e.semi_major_axis()).unwrap_or_else(|| Length::new(1.496e11));
                (default_star_id, default_mass, sma)
            }
        };

        let parent_pos = parent_id
            .and_then(|pid| positions.get(&pid).copied())
            .unwrap_or_else(Position::zero);

        let rel_pos = Position::from_raw(planet_pos.raw() - parent_pos.raw());
        let soi_radius = laplace_sphere_of_influence_radius(sma, planet.mass(), parent_mass);
        let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));

        let mut body_soi = CelestialBodySoi::new_with_geometry(
            planet.id(),
            parent_id,
            rel_pos,
            planet.mass(),
            soi_radius,
            eq_radius,
        );

        if let Ok(Some(atm)) = atmosphere_repository::get_by_planet_id(pool, &planet.id()).await {
            let surface_temp = Temperature::new(288.15);
            let mu = gravitational_parameter(planet.mass());
            let surface_g = surface_gravity(mu, eq_radius);
            if let Ok(scale_h) = atm.scale_height(surface_g, surface_temp) {
                let boundary_h = Length::new(scale_h.value() * 12.0);
                let molar_mass = atm.mean_molar_mass().unwrap_or(astronomicon_core::units::MolarMass::new(0.02897));
                let surface_density = ideal_gas_density(atm.surface_pressure(), molar_mass, surface_temp);
                body_soi = body_soi.with_atmosphere(
                    eq_radius,
                    boundary_h,
                    scale_h,
                    surface_density,
                );
            }
        }

        result.push(body_soi);
    }

    Ok(result)
}

pub async fn resolve_active_soi_for_position(
    pool: &SqlitePool,
    system_id: Uuid,
    vehicle_position: Position,
    total_epoch: Duration,
    current_parent_id: Uuid,
) -> RocketResult<Uuid> {
    let soi_bodies = resolve_system_soi_bodies(pool, system_id, total_epoch).await?;
    let positions = astronomicon_app::ephemeris::resolve_system_positions(pool, system_id, total_epoch).await?;

    let mut selected_id = current_parent_id;
    let mut min_radius = f64::INFINITY;

    for body in &soi_bodies {
        let body_pos_system = positions.get(&body.id()).copied().unwrap_or_else(Position::zero);
        let dist = (vehicle_position.raw() - body_pos_system.raw()).magnitude();
        if dist <= body.soi_radius().value() && body.soi_radius().value() < min_radius {
            min_radius = body.soi_radius().value();
            selected_id = body.id();
        }
    }

    Ok(selected_id)
}
