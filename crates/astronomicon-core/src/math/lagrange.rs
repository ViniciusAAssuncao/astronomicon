use crate::domain::OrbitalElements;
use crate::error::{DomainError, DomainResult};
use crate::units::constants::ROUTH_CRITICAL_MASS_PARAMETER;
use crate::units::{Angle, Mass, Position, Vector3};
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum LagrangePoint {
    L1,
    L2,
    L3,
    L4,
    L5,
}

pub fn solve_l1_gamma(mass_ratio: f64) -> DomainResult<f64> {
    if mass_ratio <= 0.0 || mass_ratio >= 1.0 || !mass_ratio.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "mass_ratio".to_string(),
            reason: "mass ratio must be strictly between 0 and 1".to_string(),
        });
    }

    let mu = mass_ratio;
    let mut gamma = (mu / 3.0).cbrt();
    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let g2 = gamma * gamma;
        let g3 = g2 * gamma;
        let g4 = g3 * gamma;
        let g5 = g4 * gamma;

        let f = g5 - (3.0 - mu) * g4 + (3.0 - 2.0 * mu) * g3 - mu * g2 + 2.0 * mu * gamma - mu;
        let f_prime = 5.0 * g4 - 4.0 * (3.0 - mu) * g3 + 3.0 * (3.0 - 2.0 * mu) * g2
            - 2.0 * mu * gamma
            + 2.0 * mu;

        let delta = f / f_prime;
        gamma -= delta;

        if delta.abs() < tolerance {
            return Ok(gamma);
        }
    }

    Err(DomainError::NumericalConvergence {
        context: "l1_quintic_solver".to_string(),
        reason: "failed to converge within maximum iterations".to_string(),
    })
}

pub fn solve_l2_gamma(mass_ratio: f64) -> DomainResult<f64> {
    if mass_ratio <= 0.0 || mass_ratio >= 1.0 || !mass_ratio.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "mass_ratio".to_string(),
            reason: "mass ratio must be strictly between 0 and 1".to_string(),
        });
    }

    let mu = mass_ratio;
    let mut gamma = (mu / 3.0).cbrt();
    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let g2 = gamma * gamma;
        let g3 = g2 * gamma;
        let g4 = g3 * gamma;
        let g5 = g4 * gamma;

        let f = g5 + (3.0 - mu) * g4 + (3.0 - 2.0 * mu) * g3 - mu * g2 - 2.0 * mu * gamma - mu;
        let f_prime = 5.0 * g4 + 4.0 * (3.0 - mu) * g3 + 3.0 * (3.0 - 2.0 * mu) * g2
            - 2.0 * mu * gamma
            - 2.0 * mu;

        let delta = f / f_prime;
        gamma -= delta;

        if delta.abs() < tolerance {
            return Ok(gamma);
        }
    }

    Err(DomainError::NumericalConvergence {
        context: "l2_quintic_solver".to_string(),
        reason: "failed to converge within maximum iterations".to_string(),
    })
}

pub fn solve_l3_gamma(mass_ratio: f64) -> DomainResult<f64> {
    if mass_ratio <= 0.0 || mass_ratio >= 1.0 || !mass_ratio.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "mass_ratio".to_string(),
            reason: "mass ratio must be strictly between 0 and 1".to_string(),
        });
    }

    let mu = mass_ratio;
    let mut gamma = 1.0 + 5.0 * mu / 12.0;
    let max_iter = 100;
    let tolerance = 1e-10;

    for _ in 0..max_iter {
        let g2 = gamma * gamma;
        let g3 = g2 * gamma;
        let g4 = g3 * gamma;
        let g5 = g4 * gamma;

        let f = g5 + (2.0 + mu) * g4 + (1.0 + 2.0 * mu) * g3
            - (1.0 - mu) * g2
            - 2.0 * (1.0 - mu) * gamma
            - (1.0 - mu);
        let f_prime = 5.0 * g4 + 4.0 * (2.0 + mu) * g3 + 3.0 * (1.0 + 2.0 * mu) * g2
            - 2.0 * (1.0 - mu) * gamma
            - 2.0 * (1.0 - mu);

        let delta = f / f_prime;
        gamma -= delta;

        if delta.abs() < tolerance {
            return Ok(gamma);
        }
    }

    Err(DomainError::NumericalConvergence {
        context: "l3_quintic_solver".to_string(),
        reason: "failed to converge within maximum iterations".to_string(),
    })
}

pub fn collinear_point_position(
    point: LagrangePoint,
    primary_pos: Position,
    secondary_pos: Position,
    gamma: f64,
) -> Position {
    let r1 = primary_pos.raw();
    let r2 = secondary_pos.raw();
    let r12 = r2 - r1;

    let pos = match point {
        LagrangePoint::L1 => r2 - r12 * gamma,
        LagrangePoint::L2 => r2 + r12 * gamma,
        LagrangePoint::L3 => r1 - r12 * gamma,
        LagrangePoint::L4 | LagrangePoint::L5 => r1,
    };

    Position::from_raw(pos)
}

pub fn lagrange_point_position(
    point: LagrangePoint,
    primary_pos: Position,
    secondary_pos: Position,
    mass_primary: Mass,
    mass_secondary: Mass,
    orbital_normal: Vector3,
) -> DomainResult<Position> {
    let m1 = mass_primary.value();
    let m2 = mass_secondary.value();

    if m1 <= 0.0 || !m1.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "mass_primary".to_string(),
            reason: "must be positive and finite".to_string(),
        });
    }

    if m2 <= 0.0 || !m2.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "mass_secondary".to_string(),
            reason: "must be positive and finite".to_string(),
        });
    }

    let r1 = primary_pos.raw();
    let r2 = secondary_pos.raw();
    let r12 = r2 - r1;
    let distance = r12.magnitude();

    if distance <= 0.0 || !distance.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "separation".to_string(),
            reason: "distance between primary and secondary must be positive and finite"
                .to_string(),
        });
    }

    let mass_ratio = m2 / (m1 + m2);

    match point {
        LagrangePoint::L1 => {
            let gamma = solve_l1_gamma(mass_ratio)?;
            Ok(collinear_point_position(
                LagrangePoint::L1,
                primary_pos,
                secondary_pos,
                gamma,
            ))
        }
        LagrangePoint::L2 => {
            let gamma = solve_l2_gamma(mass_ratio)?;
            Ok(collinear_point_position(
                LagrangePoint::L2,
                primary_pos,
                secondary_pos,
                gamma,
            ))
        }
        LagrangePoint::L3 => {
            let gamma = solve_l3_gamma(mass_ratio)?;
            Ok(collinear_point_position(
                LagrangePoint::L3,
                primary_pos,
                secondary_pos,
                gamma,
            ))
        }
        LagrangePoint::L4 => {
            if orbital_normal.magnitude() < 1e-12
                || !orbital_normal.0.is_finite()
                || !orbital_normal.1.is_finite()
                || !orbital_normal.2.is_finite()
            {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_normal".to_string(),
                    reason: "orbital normal must be non-zero and finite".to_string(),
                });
            }
            let normal = orbital_normal.normalized();
            let angle_l4 = PI / 3.0;
            let r_l4 = r1 + r12.rotate_about_axis(normal, angle_l4);
            Ok(Position::from_raw(r_l4))
        }
        LagrangePoint::L5 => {
            if orbital_normal.magnitude() < 1e-12
                || !orbital_normal.0.is_finite()
                || !orbital_normal.1.is_finite()
                || !orbital_normal.2.is_finite()
            {
                return Err(DomainError::InvalidInvariant {
                    field: "orbital_normal".to_string(),
                    reason: "orbital normal must be non-zero and finite".to_string(),
                });
            }
            let normal = orbital_normal.normalized();
            let angle_l5 = -PI / 3.0;
            let r_l5 = r1 + r12.rotate_about_axis(normal, angle_l5);
            Ok(Position::from_raw(r_l5))
        }
    }
}

pub fn is_lagrange_point_stable(
    point: LagrangePoint,
    mass_primary: Mass,
    mass_secondary: Mass,
) -> bool {
    match point {
        LagrangePoint::L1 | LagrangePoint::L2 | LagrangePoint::L3 => false,
        LagrangePoint::L4 | LagrangePoint::L5 => is_l4_l5_stable(mass_primary, mass_secondary),
    }
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
