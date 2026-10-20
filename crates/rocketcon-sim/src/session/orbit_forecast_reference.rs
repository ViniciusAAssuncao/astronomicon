use super::orbit_reference_curve::closed_osculating_reference;
use astronomicon_app::ephemeris::EphemerisCatalog;
use astronomicon_core::math::gravity::gravitational_parameter;
use astronomicon_core::units::{Duration, Length, Position, VelocityVector};
use rocketcon_core::math::orbital::cartesian_to_osculating_elements;
use uuid::Uuid;

pub(super) fn osculating_arc_reference(
    catalog: &EphemerisCatalog,
    epoch: Duration,
    initial_reference: Uuid,
    arc_reference: Uuid,
    world_position: Position,
    world_velocity: VelocityVector,
) -> Vec<[f64; 3]> {
    let Ok(states) = catalog.states(epoch) else {
        return Vec::new();
    };
    let Some(body) = states.get(&arc_reference) else {
        return Vec::new();
    };
    let Some(origin) = states.get(&initial_reference) else {
        return Vec::new();
    };
    let Some(mu) = catalog.mass_of_body(arc_reference).map(gravitational_parameter) else {
        return Vec::new();
    };
    let relative_position = Position::from_raw(world_position.raw() - body.position.raw());
    let relative_velocity =
        VelocityVector::from_raw(world_velocity.raw() - body.velocity.raw());
    let Ok(elements) = cartesian_to_osculating_elements(relative_position, relative_velocity, mu)
    else {
        return Vec::new();
    };
    let body_radius = catalog.radius_of_body(arc_reference).map(Length::new);
    let Some(points) = closed_osculating_reference(
        relative_position,
        relative_velocity,
        mu,
        &elements,
        body_radius,
    ) else {
        return Vec::new();
    };
    let offset = body.position.raw() - origin.position.raw();
    points
        .into_iter()
        .map(|point| [point[0] + offset.0, point[1] + offset.1, point[2] + offset.2])
        .collect()
}
