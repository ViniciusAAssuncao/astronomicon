use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::state_properties::laplace_runge_lenz_vector;
use crate::math::orbital::types::{OrbitType, OsculatingElements};
use astronomicon_core::units::{
    Angle, GravitationalParameter, Length, Position, Vector3, VelocityVector,
};
use std::f64::consts::TAU;

pub fn state_vectors_to_keplerian(
    position: Vector3,
    velocity: Vector3,
    mu: f64,
) -> RocketDomainResult<OsculatingElements> {
    if mu <= 0.0 || !mu.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "mu".to_string(),
            reason: "gravitational parameter must be positive and finite".to_string(),
        });
    }

    let r = position.magnitude();
    let v = velocity.magnitude();

    if r <= 1e-6 || !r.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "position".to_string(),
            reason: "position magnitude must be positive and finite".to_string(),
        });
    }

    if !v.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "velocity".to_string(),
            reason: "velocity must be finite".to_string(),
        });
    }

    let h_vec = position.cross(&velocity);
    let h = h_vec.magnitude();

    let energy = 0.5 * v * v - mu / r;

    let e_vec = laplace_runge_lenz_vector(position, velocity, mu);
    let e = e_vec.magnitude();

    let orbit_type = if (1.0 - e).abs() < 1e-6 || (energy.abs() < 1e-12 && (1.0 - e).abs() < 0.05) {
        OrbitType::Parabolic
    } else if e < 1e-6 {
        OrbitType::Circular
    } else if e < 1.0 {
        OrbitType::Elliptic
    } else {
        OrbitType::Hyperbolic
    };

    let semi_major_axis = match orbit_type {
        OrbitType::Parabolic => Length::new(f64::INFINITY),
        OrbitType::Circular | OrbitType::Elliptic => {
            if energy < 0.0 {
                Length::new(-mu / (2.0 * energy))
            } else {
                let p = (h * h) / mu;
                Length::new(p / (1.0 - e * e).max(1e-12))
            }
        }
        OrbitType::Hyperbolic => {
            if energy > 0.0 {
                Length::new(-mu / (2.0 * energy))
            } else {
                let p = (h * h) / mu;
                Length::new(-p / (e * e - 1.0).max(1e-12))
            }
        }
    };

    let p = if h > 1e-12 {
        (h * h) / mu
    } else if orbit_type == OrbitType::Parabolic {
        r * (1.0 + position.dot(&velocity).signum())
    } else {
        semi_major_axis.value().abs() * (1.0 - e * e).abs()
    };

    let periapsis_distance = if e == 1.0 || orbit_type == OrbitType::Parabolic {
        Length::new(p * 0.5)
    } else if e < 1.0 {
        Length::new(semi_major_axis.value() * (1.0 - e))
    } else {
        Length::new(-semi_major_axis.value() * (e - 1.0))
    };

    let apoapsis_distance = if e < 1.0 {
        Some(Length::new(semi_major_axis.value() * (1.0 + e)))
    } else {
        None
    };

    let inc_val = if h > 1e-12 {
        (h_vec.2 / h).clamp(-1.0, 1.0).acos()
    } else {
        0.0
    };
    let inclination = Angle::new(inc_val);

    let n_vec = Vector3::new(-h_vec.1, h_vec.0, 0.0);
    let n = n_vec.magnitude();

    let raan_val = if n > 1e-10 {
        n_vec.1.atan2(n_vec.0).rem_euclid(TAU)
    } else {
        0.0
    };
    let longitude_of_ascending_node = Angle::new(raan_val);

    let arg_peri_val = if n > 1e-10 && e > 1e-8 {
        let cos_w = (n_vec.dot(&e_vec) / (n * e)).clamp(-1.0, 1.0);
        let sin_w = ((n_vec.cross(&e_vec)).dot(&h_vec) / (n * e * h)).clamp(-1.0, 1.0);
        sin_w.atan2(cos_w).rem_euclid(TAU)
    } else if n <= 1e-10 && e > 1e-8 {
        if h_vec.2 >= 0.0 {
            e_vec.1.atan2(e_vec.0).rem_euclid(TAU)
        } else {
            (-e_vec.1).atan2(e_vec.0).rem_euclid(TAU)
        }
    } else {
        0.0
    };
    let argument_of_periapsis = Angle::new(arg_peri_val);

    let true_anom_val = if e > 1e-8 {
        let cos_nu = (e_vec.dot(&position) / (e * r)).clamp(-1.0, 1.0);
        let sin_nu = ((e_vec.cross(&position)).dot(&h_vec) / (e * r * h.max(1e-12))).clamp(-1.0, 1.0);
        let nu_raw = sin_nu.atan2(cos_nu);
        if e < 1.0 {
            nu_raw.rem_euclid(TAU)
        } else {
            nu_raw
        }
    } else if n > 1e-10 {
        let cos_u = (n_vec.dot(&position) / (n * r)).clamp(-1.0, 1.0);
        let sin_u = ((n_vec.cross(&position)).dot(&h_vec) / (n * r * h.max(1e-12))).clamp(-1.0, 1.0);
        sin_u.atan2(cos_u).rem_euclid(TAU)
    } else if h_vec.2 >= 0.0 {
        position.1.atan2(position.0).rem_euclid(TAU)
    } else {
        (-position.1).atan2(position.0).rem_euclid(TAU)
    };
    let true_anomaly = Angle::new(true_anom_val);

    Ok(OsculatingElements::new(
        semi_major_axis,
        e,
        inclination,
        longitude_of_ascending_node,
        argument_of_periapsis,
        true_anomaly,
        periapsis_distance,
        apoapsis_distance,
        energy,
        h_vec,
        orbit_type,
    ))
}

pub fn cartesian_to_osculating_elements(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
) -> RocketDomainResult<OsculatingElements> {
    state_vectors_to_keplerian(position.raw(), velocity.raw(), mu.value())
}

pub fn cartesian_to_keplerian(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
) -> RocketDomainResult<OsculatingElements> {
    cartesian_to_osculating_elements(position, velocity, mu)
}

pub fn osculating_elements_to_cartesian(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
) -> RocketDomainResult<(Position, VelocityVector)> {
    let mu_val = mu.value();
    if mu_val <= 0.0 || !mu_val.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "mu".to_string(),
            reason: "gravitational parameter must be positive and finite".to_string(),
        });
    }

    let e = elements.eccentricity;
    let a = elements.semi_major_axis.value();
    let nu = elements.true_anomaly.value();
    let inc = elements.inclination.value();
    let raan = elements.longitude_of_ascending_node.value();
    let omega = elements.argument_of_periapsis.value();

    let p = if elements.orbit_type == OrbitType::Parabolic || (1.0 - e).abs() < 1e-6 {
        2.0 * elements.periapsis_distance.value()
    } else if e < 1.0 {
        a * (1.0 - e * e)
    } else {
        (-a) * (e * e - 1.0)
    };

    if p <= 0.0 || !p.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "semi_latus_rectum".to_string(),
            reason: "semi-latus rectum must be positive and finite".to_string(),
        });
    }

    let denom = 1.0 + e * nu.cos();
    if denom <= 1e-12 || !denom.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "true_anomaly".to_string(),
            reason: "true anomaly results in non-physical infinite or undefined radial distance"
                .to_string(),
        });
    }

    let r = p / denom;
    let r_pqw = Vector3::new(r * nu.cos(), r * nu.sin(), 0.0);

    let h = (mu_val * p).sqrt();
    let v_factor = mu_val / h;
    let v_pqw = Vector3::new(-v_factor * nu.sin(), v_factor * (e + nu.cos()), 0.0);

    let r_inertial = r_pqw
        .rotate_about_z(omega)
        .rotate_about_x(inc)
        .rotate_about_z(raan);

    let v_inertial = v_pqw
        .rotate_about_z(omega)
        .rotate_about_x(inc)
        .rotate_about_z(raan);

    Ok((
        Position::from_raw(r_inertial),
        VelocityVector::from_raw(v_inertial),
    ))
}

pub fn keplerian_to_cartesian(
    elements: &OsculatingElements,
    mu: GravitationalParameter,
) -> RocketDomainResult<(Position, VelocityVector)> {
    osculating_elements_to_cartesian(elements, mu)
}