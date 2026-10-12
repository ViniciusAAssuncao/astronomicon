use crate::environment::snapshot::EnvironmentSnapshot;
use astronomicon_core::math::gravity::gravitational_acceleration_at;
use astronomicon_core::units::{AccelerationVector, Position};

pub fn gravitational_acceleration_at_snapshot(
    snapshot: &EnvironmentSnapshot,
    point: Position,
) -> AccelerationVector {
    let mut sources = vec![(snapshot.star_position, snapshot.star.mass())];
    if let (Some(planet), Some(position)) = (&snapshot.planet, snapshot.planet_position) {
        sources.push((position, planet.mass()));
    }
    gravitational_acceleration_at(point, &sources)
}
