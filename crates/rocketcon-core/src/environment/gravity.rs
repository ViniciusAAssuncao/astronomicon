use crate::environment::snapshot::EnvironmentSnapshot;
use astronomicon_core::math::gravity::gravitational_acceleration_at;
use astronomicon_core::units::{AccelerationVector, Position};

pub fn gravitational_acceleration_at_snapshot(
    snapshot: &EnvironmentSnapshot,
    point: Position,
) -> AccelerationVector {
    let sources = [
        (snapshot.star_position, snapshot.star.mass()),
        (snapshot.planet_position, snapshot.planet.mass()),
    ];
    gravitational_acceleration_at(point, &sources)
}
