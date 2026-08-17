use crate::domain::OrbitalElements;
use crate::error::DomainResult;
use crate::units::constants::ROUTH_CRITICAL_MASS_PARAMETER;
use crate::units::{Angle, Mass, Position, Vector3};
use std::f64::consts::PI;

pub fn lagrange_points_l4_l5(
    primary_pos: Position,
    secondary_pos: Position,
    orbital_normal: Vector3,
) -> (Position, Position) {
    let r1 = primary_pos.raw();
    let r2 = secondary_pos.raw();
    let r12 = r2 - r1;
    let normal = orbital_normal.normalized();

    let angle_l4 = PI / 3.0;
    let angle_l5 = -PI / 3.0;

    let r_l4 = r1 + r12.rotate_about_axis(normal, angle_l4);
    let r_l5 = r1 + r12.rotate_about_axis(normal, angle_l5);

    (Position::from_raw(r_l4), Position::from_raw(r_l5))
}

pub fn orbital_plane_normal(elements: &OrbitalElements) -> Vector3 {
    let inc = elements.inclination().value();
    let raan = elements.longitude_of_ascending_node().value();
    Vector3::new(0.0, 0.0, 1.0)
        .rotate_about_x(inc)
        .rotate_about_z(raan)
}

pub fn orbital_normal_from_vectors(relative_pos: Vector3, relative_vel: Vector3) -> Vector3 {
    relative_pos.cross(&relative_vel).normalized()
}

pub fn is_l4_l5_stable(mass_primary: Mass, mass_secondary: Mass) -> bool {
    let m1 = mass_primary.value();
    let m2 = mass_secondary.value();
    let total = m1 + m2;
    if total <= 0.0 {
        return false;
    }
    let smaller_mass = m1.min(m2);
    let mu = smaller_mass / total;
    mu < ROUTH_CRITICAL_MASS_PARAMETER
}

pub fn co_orbital_elements(
    host_elements: &OrbitalElements,
    mean_anomaly_offset: Angle,
) -> DomainResult<OrbitalElements> {
    OrbitalElements::new(
        host_elements.semi_major_axis(),
        host_elements.eccentricity(),
        host_elements.inclination(),
        host_elements.longitude_of_ascending_node(),
        host_elements.argument_of_periapsis(),
        host_elements.mean_anomaly_at_epoch() + mean_anomaly_offset,
    )
}

pub fn trojan_elements(
    host_elements: &OrbitalElements,
    is_l4: bool,
) -> DomainResult<OrbitalElements> {
    let offset = if is_l4 {
        Angle::new(PI / 3.0)
    } else {
        Angle::new(-PI / 3.0)
    };
    co_orbital_elements(host_elements, offset)
}