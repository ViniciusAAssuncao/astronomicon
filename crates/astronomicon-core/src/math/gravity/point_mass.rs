use crate::units::constants::GRAVITATIONAL_CONSTANT;
use crate::units::{
    Acceleration, AccelerationVector, GravitationalParameter, Length, Mass, Position, Vector3,
};

pub fn gravitational_parameter(mass: Mass) -> GravitationalParameter {
    GravitationalParameter::new(GRAVITATIONAL_CONSTANT * mass.value())
}

pub fn combined_gravitational_parameter(mass_a: Mass, mass_b: Mass) -> GravitationalParameter {
    gravitational_parameter(mass_a + mass_b)
}

pub fn surface_gravity(mu: GravitationalParameter, radius: Length) -> Acceleration {
    if radius.value() <= 0.0 {
        Acceleration::new(0.0)
    } else {
        Acceleration::new(mu.value() / (radius.value() * radius.value()))
    }
}

pub fn gravitational_acceleration_at_altitude(
    mu: GravitationalParameter,
    equatorial_radius: Length,
    altitude: Length,
) -> Acceleration {
    let r = equatorial_radius.value() + altitude.value();
    if r <= 0.0 {
        Acceleration::new(0.0)
    } else {
        Acceleration::new(mu.value() / (r * r))
    }
}

pub fn gravitational_acceleration_at(
    point: Position,
    sources: &[(Position, Mass)],
) -> AccelerationVector {
    let mut total_acc = Vector3::zero();
    let p = point.raw();

    for (pos, mass) in sources {
        let diff = pos.raw() - p;
        let dist = diff.magnitude();
        if dist > 1e-6 && mass.value() > 0.0 {
            let factor = GRAVITATIONAL_CONSTANT * mass.value() / (dist * dist * dist);
            total_acc = total_acc + diff * factor;
        }
    }

    AccelerationVector::from_raw(total_acc)
}
