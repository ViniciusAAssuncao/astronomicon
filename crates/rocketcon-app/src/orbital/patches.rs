use crate::error::{RocketError, RocketResult};
use crate::orbital::orbit::resolve_relative_state_for_body;
use crate::orbital::soi::{resolve_body_state_at_epoch, resolve_system_soi_bodies};
use astronomicon_core::domain::{Planet, Star};
use astronomicon_core::math::atmosphere::ideal_gas_density;
use astronomicon_core::math::gravity::{gravitational_parameter, surface_gravity};
use astronomicon_core::units::{Duration, Length, Position, Temperature, VelocityVector};
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{atmosphere_repository, planet_repository, star_repository};
use rocketcon_core::domain::{TrajectoryPatch, VehiclePhysicalState};
use rocketcon_core::math::orbital::sphere_of_influence::CelestialBodySoi;
use rocketcon_core::math::orbital::trajectory_prediction::compute_conic_patches;
use rocketcon_db::repositories::{
    trajectory_patch as trajectory_patch_repository,
    vehicle_physical_state as vehicle_physical_state_repository,
};
use uuid::Uuid;

pub async fn resolve_active_trajectory_patch(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<Option<TrajectoryPatch>> {
    let total_epoch = universe_epoch + at_epoch;
    let patches = trajectory_patch_repository::list_for_vehicle(pool, &vehicle_id).await?;
    let active = patches.into_iter().find(|p| p.is_active_at(total_epoch));
    Ok(active)
}

pub async fn invalidate_vehicle_trajectory_patches(
    pool: &SqlitePool,
    vehicle_id: Uuid,
) -> RocketResult<()> {
    trajectory_patch_repository::delete_patches_for_vehicle(pool, &vehicle_id).await?;
    Ok(())
}

pub async fn invalidate_future_trajectory_patches(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    from_epoch: Duration,
) -> RocketResult<()> {
    trajectory_patch_repository::delete_future_patches_after_epoch(pool, &vehicle_id, from_epoch).await?;
    Ok(())
}

pub async fn generate_and_save_trajectory_patches(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
) -> RocketResult<Vec<TrajectoryPatch>> {
    let physical_state = vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let total_epoch = universe_epoch + at_epoch;
    let ref_id = physical_state.reference_body_id();

    let (system_id, current_body) = if let Some(p_row) = planet_repository::get_by_id(pool, &ref_id).await? {
        let planet = Planet::try_from(p_row)?;
        let star = astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
        let sys_id = star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no system", star.id()))
        })?;
        let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
        let soi_radius = Length::new(eq_radius.value() * 100.0);
        let mut cb = CelestialBodySoi::new_with_geometry(planet.id(), Some(star.id()), Position::zero(), planet.mass(), soi_radius, eq_radius);
        if let Some(atm) = atmosphere_repository::get_by_planet_id(pool, &planet.id()).await? {
            let surface_temp = Temperature::new(288.15);
            let mu = gravitational_parameter(planet.mass());
            let surface_g = surface_gravity(mu, eq_radius);
            if let Ok(scale_h) = atm.scale_height(surface_g, surface_temp) {
                let boundary_h = Length::new(scale_h.value() * 12.0);
                let molar_mass = atm.mean_molar_mass().unwrap_or(astronomicon_core::units::MolarMass::new(0.02897));
                let surface_density = ideal_gas_density(atm.surface_pressure(), molar_mass, surface_temp);
                cb = cb.with_atmosphere(eq_radius, boundary_h, scale_h, surface_density);
            }
        }
        (sys_id, cb)
    } else if let Some(s_row) = star_repository::get_by_id(pool, &ref_id).await? {
        let star = Star::try_from(s_row)?;
        let sys_id = star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("star '{}' has no system", star.id()))
        })?;
        let s_radius = star.radius().unwrap_or_else(|| Length::new(6.957e8));
        let cb = CelestialBodySoi::new_with_geometry(star.id(), None, Position::zero(), star.mass(), Length::new(f64::INFINITY), s_radius);
        (sys_id, cb)
    } else {
        return Err(RocketError::Generic(format!("reference body '{}' not found", ref_id)));
    };

    let soi_bodies = resolve_system_soi_bodies(pool, system_id, total_epoch).await?;
    let matched_body = soi_bodies
        .iter()
        .find(|b| b.id() == ref_id)
        .cloned()
        .unwrap_or(current_body);

    let (rel_pos, rel_vel, mu) = resolve_relative_state_for_body(pool, &physical_state, ref_id, system_id, total_epoch).await?;

    let max_lookahead = Duration::new(86400.0 * 365.25 * 3.0);
    let patches = compute_conic_patches(
        vehicle_id,
        (rel_pos, rel_vel),
        &matched_body,
        mu,
        &soi_bodies,
        total_epoch,
        6,
        max_lookahead,
    )?;

    trajectory_patch_repository::delete_patches_for_vehicle(pool, &vehicle_id).await?;
    trajectory_patch_repository::insert_patches(pool, &patches).await?;

    Ok(patches)
}

pub async fn propagate_coasting_vehicle(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    dt: Duration,
    universe_epoch: Duration,
    current_at_epoch: Duration,
) -> RocketResult<VehiclePhysicalState> {
    let physical_state = vehicle_physical_state_repository::get_by_vehicle_id(pool, &vehicle_id)
        .await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let current_total_epoch = universe_epoch + current_at_epoch;
    let target_total_epoch = current_total_epoch + dt;
    let new_at_epoch = current_at_epoch + dt;

    let patch = match resolve_active_trajectory_patch(pool, vehicle_id, universe_epoch, current_at_epoch).await? {
        Some(p) => p,
        None => {
            let patches = generate_and_save_trajectory_patches(pool, vehicle_id, universe_epoch, current_at_epoch).await?;
            patches
                .into_iter()
                .find(|p| p.is_active_at(current_total_epoch))
                .ok_or_else(|| {
                    RocketError::Generic(format!("failed to generate valid trajectory patch for vehicle '{}'", vehicle_id))
                })?
        }
    };

    let active_patch = if patch.is_active_at(target_total_epoch) {
        patch
    } else {
        let patches = trajectory_patch_repository::list_for_vehicle(pool, &vehicle_id).await?;
        match patches.into_iter().find(|p| p.is_active_at(target_total_epoch)) {
            Some(next_patch) => next_patch,
            None => {
                let regenerated = generate_and_save_trajectory_patches(pool, vehicle_id, universe_epoch, current_at_epoch).await?;
                regenerated
                    .into_iter()
                    .find(|p| p.is_active_at(target_total_epoch))
                    .unwrap_or(patch)
            }
        }
    };

    let ref_id = active_patch.reference_body_id();
    let (rel_pos, rel_vel) = active_patch.evaluate_state_at(target_total_epoch)?;

    let system_id = if let Some(p_row) = planet_repository::get_by_id(pool, &ref_id).await? {
        let planet = Planet::try_from(p_row)?;
        let star = astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
        star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no system", star.id()))
        })?
    } else if let Some(s_row) = star_repository::get_by_id(pool, &ref_id).await? {
        let star = Star::try_from(s_row)?;
        star.star_system_id().ok_or_else(|| {
            RocketError::Generic(format!("star '{}' has no system", star.id()))
        })?
    } else {
        return Err(RocketError::Generic(format!("reference body '{}' not found", ref_id)));
    };

    let (body_pos, body_vel) = resolve_body_state_at_epoch(pool, ref_id, system_id, target_total_epoch).await?;
    let new_pos = Position::from_raw(body_pos.raw() + rel_pos.raw());
    let new_vel = VelocityVector::from_raw(body_vel.raw() + rel_vel.raw());

    let new_orientation = physical_state.orientation().integrate(physical_state.angular_velocity(), dt);

    let new_physical_state = VehiclePhysicalState::new_with_max_q(
        vehicle_id,
        new_pos,
        new_vel,
        new_orientation,
        physical_state.angular_velocity(),
        ref_id,
        universe_epoch,
        new_at_epoch,
        physical_state.max_dynamic_pressure(),
        physical_state.max_dynamic_pressure_epoch(),
    )?;

    vehicle_physical_state_repository::upsert(pool, &new_physical_state).await?;

    Ok(new_physical_state)
}
