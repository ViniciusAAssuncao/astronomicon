use super::frames::local_to_inertial_delta_v;
use super::types::ManeuverDeltaV;
use crate::error::RocketDomainResult;
use crate::math::orbital::conversions::cartesian_to_osculating_elements;
use crate::math::orbital::types::OsculatingElements;
use astronomicon_core::units::{GravitationalParameter, Position, VelocityVector};

pub fn apply_impulsive_delta_v(
    initial_velocity: VelocityVector,
    delta_v: VelocityVector,
) -> VelocityVector {
    VelocityVector::from_raw(initial_velocity.raw() + delta_v.raw())
}

pub fn apply_local_maneuver(
    position: Position,
    initial_velocity: VelocityVector,
    maneuver: ManeuverDeltaV,
) -> VelocityVector {
    let dv_inertial = local_to_inertial_delta_v(maneuver, position, initial_velocity);
    apply_impulsive_delta_v(initial_velocity, dv_inertial)
}

pub fn orbit_after_impulsive_burn(
    position: Position,
    initial_velocity: VelocityVector,
    delta_v: VelocityVector,
    mu: GravitationalParameter,
) -> RocketDomainResult<OsculatingElements> {
    let v_new = apply_impulsive_delta_v(initial_velocity, delta_v);
    cartesian_to_osculating_elements(position, v_new, mu)
}

pub fn orbit_after_local_maneuver(
    position: Position,
    initial_velocity: VelocityVector,
    maneuver: ManeuverDeltaV,
    mu: GravitationalParameter,
) -> RocketDomainResult<OsculatingElements> {
    let v_new = apply_local_maneuver(position, initial_velocity, maneuver);
    cartesian_to_osculating_elements(position, v_new, mu)
}
