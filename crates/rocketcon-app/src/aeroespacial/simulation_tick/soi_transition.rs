use crate::error::RocketResult;
use crate::orbital::invalidate_vehicle_trajectory_patches;
use astronomicon_core::domain::{OrbitalParent, Planet};
use astronomicon_core::units::Position;
use astronomicon_db::SqlitePool;
use astronomicon_db::repositories::planet_repository;
use rocketcon_core::domain::VehiclePhysicalState;
use rocketcon_core::environment::EnvironmentSnapshot;
use rocketcon_core::math::orbital::sphere_of_influence::laplace_sphere_of_influence_radius;
use rocketcon_db::repositories::vehicle_physical_state as physical_repository;
use std::collections::HashMap;
use uuid::Uuid;

pub(super) async fn apply_confirmed_soi_transition(
    pool: &SqlitePool,
    next: VehiclePhysicalState,
    environment: &EnvironmentSnapshot,
    positions: &HashMap<Uuid, Position>,
) -> RocketResult<(VehiclePhysicalState, bool)> {
    let target = if let Some(planet) = environment.planet.as_ref() {
        let Some(radius) =
            planet_soi_radius(planet, environment.star.id(), environment.star.mass())
        else {
            return Ok((next, false));
        };
        let Some(position) = positions.get(&planet.id()) else {
            return Ok((next, false));
        };
        let distance = (next.position().raw() - position.raw()).magnitude();
        if !distance.is_finite() || distance <= radius {
            return Ok((next, false));
        }
        Some(environment.star.id())
    } else {
        let candidates = planet_repository::list_by_system(pool, &environment.system_id).await?;
        let mut nearest = None;
        let mut smallest_radius = f64::INFINITY;
        for row in candidates {
            let planet = Planet::try_from(row)?;
            let Some(radius) =
                planet_soi_radius(&planet, environment.star.id(), environment.star.mass())
            else {
                continue;
            };
            let Some(position) = positions.get(&planet.id()) else {
                continue;
            };
            let distance = (next.position().raw() - position.raw()).magnitude();
            if distance.is_finite() && distance < radius && radius < smallest_radius {
                smallest_radius = radius;
                nearest = Some(planet.id());
            }
        }
        nearest
    };
    let Some(reference_body_id) = target else {
        return Ok((next, false));
    };
    if reference_body_id == next.reference_body_id() {
        return Ok((next, false));
    }
    let changed = VehiclePhysicalState::new_with_max_q(
        next.vehicle_id(),
        next.position(),
        next.velocity(),
        next.orientation(),
        next.angular_velocity(),
        reference_body_id,
        next.captured_universe_epoch(),
        next.captured_at_epoch(),
        next.max_dynamic_pressure(),
        next.max_dynamic_pressure_epoch(),
    )?;
    physical_repository::upsert(pool, &changed).await?;
    invalidate_vehicle_trajectory_patches(pool, changed.vehicle_id()).await?;
    Ok((changed, true))
}

fn planet_soi_radius(
    planet: &Planet,
    parent_id: Uuid,
    parent_mass: astronomicon_core::units::Mass,
) -> Option<f64> {
    if !matches!(planet.orbital_parent(), OrbitalParent::Star(id) if id == parent_id) {
        return None;
    }
    let elements = planet.orbital_elements()?;
    let radius =
        laplace_sphere_of_influence_radius(elements.semi_major_axis(), planet.mass(), parent_mass)
            .value();
    (radius.is_finite() && radius > 0.0).then_some(radius)
}
