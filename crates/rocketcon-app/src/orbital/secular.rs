use crate::aeroespacial::resolve_vehicle_real_mass;
use crate::error::{ RocketError, RocketResult };
use crate::orbital::orbit::resolve_relative_state_for_body;
use crate::orbital::patches::invalidate_future_trajectory_patches;
use astronomicon_core::domain::Planet;
use astronomicon_core::units::{ Density, Duration, Length, Position, VelocityVector };
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::{ atmosphere_repository, planet_repository };
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::math::mass_properties::resolve_vehicle_optical_surface_properties;
use rocketcon_core::math::orbital::conversions::cartesian_to_osculating_elements;
use rocketcon_core::math::orbital::orbital_perturbation::{
    estimate_orbit_lifetime,
    propagate_secular_orbit_decay,
    SecularOrbitDecayPrediction,
    ZonalHarmonics,
};
use rocketcon_db::repositories::{
    vehicle as vehicle_repository,
    vehicle_physical_state as vehicle_physical_state_repository,
};
use uuid::Uuid;

pub async fn resolve_vehicle_secular_decay(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    duration: Duration,
    universe_epoch: Duration,
    current_at_epoch: Duration
) -> RocketResult<SecularOrbitDecayPrediction> {
    let physical_state = vehicle_physical_state_repository
        ::get_by_vehicle_id(pool, &vehicle_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let ref_id = physical_state.reference_body_id();
    let planet_row = planet_repository
        ::get_by_id(pool, &ref_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("reference planet '{}' not found", ref_id))
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
    let system_id = star
        .star_system_id()
        .ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no system", star.id()))
        })?;

    let total_epoch = universe_epoch + current_at_epoch;
    let (rel_pos, rel_vel, mu) = resolve_relative_state_for_body(
        pool,
        &physical_state,
        ref_id,
        system_id,
        total_epoch
    ).await?;

    let elements = cartesian_to_osculating_elements(rel_pos, rel_vel, mu)?;

    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let mut stages: Vec<u32> = components
        .iter()
        .map(|(e, _)| e.stage_index())
        .collect();
    stages.sort_unstable();
    stages.dedup();
    if stages.is_empty() {
        stages.push(0);
    }

    let real_mass = resolve_vehicle_real_mass(
        pool,
        vehicle_id,
        universe_epoch,
        current_at_epoch
    ).await?;
    let optical_props = resolve_vehicle_optical_surface_properties(&components, &stages);

    let m_val = real_mass.value().max(1.0);
    let beta = optical_props.drag_area_product_m2 / m_val;

    let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let j2 = planet.oblateness_j2().unwrap_or(0.0);
    let harmonics = ZonalHarmonics::j2_only(j2);

    let (atm_rho0, atm_scale_h, atm_boundary_h) = match
        atmosphere_repository::get_by_planet_id(pool, &ref_id).await?
    {
        Some(atm) => {
            let p0 = atm.surface_pressure().value();
            let temp = 288.15;
            let mm = atm
                .mean_molar_mass()
                .map(|m| m.value())
                .unwrap_or(0.02897);
            let r_spec = astronomicon_core::units::constants::UNIVERSAL_GAS_CONSTANT / mm;
            let rho0 = p0 / (r_spec * temp);
            let g = astronomicon_core::math::gravity::surface_gravity(mu, eq_radius).value();
            let scale_h = ((r_spec * temp) / g).max(100.0);
            (Density::new(rho0), Length::new(scale_h), Length::new(scale_h * 12.0))
        }
        None => (Density::new(0.0), Length::new(8500.0), Length::new(100_000.0)),
    };

    let prediction = propagate_secular_orbit_decay(
        &elements,
        atm_rho0,
        atm_scale_h,
        atm_boundary_h,
        beta,
        eq_radius,
        mu,
        &harmonics,
        duration,
        200
    );

    Ok(prediction)
}

pub async fn propagate_and_persist_secular_decay(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    duration: Duration,
    universe_epoch: Duration,
    current_at_epoch: Duration
) -> RocketResult<VehiclePhysicalState> {
    let physical_state = vehicle_physical_state_repository
        ::get_by_vehicle_id(pool, &vehicle_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let prediction = resolve_vehicle_secular_decay(
        pool,
        vehicle_id,
        duration,
        universe_epoch,
        current_at_epoch
    ).await?;

    let ref_id = physical_state.reference_body_id();
    let planet_row = planet_repository
        ::get_by_id(pool, &ref_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("reference planet '{}' not found", ref_id))
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
    let system_id = star
        .star_system_id()
        .ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no system", star.id()))
        })?;

    let new_at_epoch = current_at_epoch + duration;
    let target_total_epoch = universe_epoch + new_at_epoch;

    let (body_pos, body_vel) = crate::orbital::soi::resolve_body_state_at_epoch(
        pool,
        ref_id,
        system_id,
        target_total_epoch
    ).await?;

    let new_pos = Position::from_raw(body_pos.raw() + prediction.final_position.raw());
    let new_vel = VelocityVector::from_raw(body_vel.raw() + prediction.final_velocity.raw());
    let new_orientation = physical_state
        .orientation()
        .integrate(physical_state.angular_velocity(), duration);

    let new_state = VehiclePhysicalState::new_with_max_q(
        vehicle_id,
        new_pos,
        new_vel,
        new_orientation,
        physical_state.angular_velocity(),
        ref_id,
        universe_epoch,
        new_at_epoch,
        physical_state.max_dynamic_pressure(),
        physical_state.max_dynamic_pressure_epoch()
    )?;

    vehicle_physical_state_repository::upsert(pool, &new_state).await?;
    invalidate_future_trajectory_patches(
        pool,
        vehicle_id,
        universe_epoch + current_at_epoch
    ).await?;

    Ok(new_state)
}

pub async fn estimate_vehicle_orbital_lifetime(
    pool: &SqlitePool,
    vehicle_id: Uuid,
    universe_epoch: Duration,
    current_at_epoch: Duration
) -> RocketResult<Option<Duration>> {
    let physical_state = vehicle_physical_state_repository
        ::get_by_vehicle_id(pool, &vehicle_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("physical state for vehicle '{}' not found", vehicle_id))
        })?;

    let ref_id = physical_state.reference_body_id();
    let planet_row = planet_repository
        ::get_by_id(pool, &ref_id).await?
        .ok_or_else(|| {
            RocketError::Generic(format!("reference planet '{}' not found", ref_id))
        })?;
    let planet = Planet::try_from(planet_row)?;

    let star = astronomicon_app::hierarchy::find_parent_star(pool, planet.orbital_parent()).await?;
    let system_id = star
        .star_system_id()
        .ok_or_else(|| {
            RocketError::Generic(format!("parent star '{}' has no system", star.id()))
        })?;

    let total_epoch = universe_epoch + current_at_epoch;
    let (rel_pos, rel_vel, mu) = resolve_relative_state_for_body(
        pool,
        &physical_state,
        ref_id,
        system_id,
        total_epoch
    ).await?;

    let elements = cartesian_to_osculating_elements(rel_pos, rel_vel, mu)?;

    let components = vehicle_repository::list_components_for_vehicle(pool, &vehicle_id).await?;
    let mut stages: Vec<u32> = components
        .iter()
        .map(|(e, _)| e.stage_index())
        .collect();
    stages.sort_unstable();
    stages.dedup();
    if stages.is_empty() {
        stages.push(0);
    }

    let real_mass = resolve_vehicle_real_mass(
        pool,
        vehicle_id,
        universe_epoch,
        current_at_epoch
    ).await?;
    let optical_props = resolve_vehicle_optical_surface_properties(&components, &stages);

    let m_val = real_mass.value().max(1.0);
    let beta = optical_props.drag_area_product_m2 / m_val;

    let eq_radius = planet.equatorial_radius().unwrap_or_else(|| Length::new(6371e3));
    let j2 = planet.oblateness_j2().unwrap_or(0.0);
    let harmonics = ZonalHarmonics::j2_only(j2);

    let (atm_rho0, atm_scale_h, atm_boundary_h) = match
        atmosphere_repository::get_by_planet_id(pool, &ref_id).await?
    {
        Some(atm) => {
            let p0 = atm.surface_pressure().value();
            let temp = 288.15;
            let mm = atm
                .mean_molar_mass()
                .map(|m| m.value())
                .unwrap_or(0.02897);
            let r_spec = astronomicon_core::units::constants::UNIVERSAL_GAS_CONSTANT / mm;
            let rho0 = p0 / (r_spec * temp);
            let g = astronomicon_core::math::gravity::surface_gravity(mu, eq_radius).value();
            let scale_h = ((r_spec * temp) / g).max(100.0);
            (Density::new(rho0), Length::new(scale_h), Length::new(scale_h * 12.0))
        }
        None => {
            return Ok(None);
        }
    };

    let lifetime = estimate_orbit_lifetime(
        &elements,
        atm_rho0,
        atm_scale_h,
        atm_boundary_h,
        beta,
        eq_radius,
        mu,
        &harmonics
    );

    Ok(lifetime)
}
