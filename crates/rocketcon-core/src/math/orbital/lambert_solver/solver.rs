use super::root_finding::brent_root_find;
use super::types::{LambertSolution, TransferDirection};
use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::types::OrbitType;
use crate::math::orbital::universal::{stumpff_c, stumpff_s};
use astronomicon_core::units::{
    Angle, Duration, GravitationalParameter, Length, Position, VelocityVector,
};
use std::f64::consts::{PI, TAU};

pub fn solve_lambert(
    r1: Position,
    r2: Position,
    time_of_flight: Duration,
    mu: GravitationalParameter,
    direction: TransferDirection,
) -> RocketDomainResult<LambertSolution> {
    let r1_vec = r1.raw();
    let r2_vec = r2.raw();
    let r1_val = r1_vec.magnitude();
    let r2_val = r2_vec.magnitude();
    let tof_val = time_of_flight.value();
    let mu_val = mu.value();

    if r1_val <= 1e-6 || !r1_val.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "r1".to_string(),
            reason: "departure position magnitude must be positive and finite".to_string(),
        });
    }

    if r2_val <= 1e-6 || !r2_val.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "r2".to_string(),
            reason: "arrival position magnitude must be positive and finite".to_string(),
        });
    }

    if tof_val <= 0.0 || !tof_val.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "time_of_flight".to_string(),
            reason: "time of flight must be positive and finite".to_string(),
        });
    }

    if mu_val <= 0.0 || !mu_val.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "mu".to_string(),
            reason: "gravitational parameter must be positive and finite".to_string(),
        });
    }

    let cos_theta = (r1_vec.dot(&r2_vec) / (r1_val * r2_val)).clamp(-1.0, 1.0);
    let theta = cos_theta.acos();

    if theta < 1e-7 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "geometry".to_string(),
            reason: "departure and arrival position vectors are collinear".to_string(),
        });
    }

    if (PI - theta).abs() < 1e-7 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "geometry".to_string(),
            reason: "departure and arrival position vectors are antipodal".to_string(),
        });
    }

    let delta_nu = match direction {
        TransferDirection::ShortWay => theta,
        TransferDirection::LongWay => TAU - theta,
    };

    let sin_delta_nu = delta_nu.sin();
    let cos_delta_nu = delta_nu.cos();
    let a_coeff = sin_delta_nu * ((r1_val * r2_val) / (1.0 - cos_delta_nu)).sqrt();

    let calc_y = |z: f64| -> f64 {
        let cz = stumpff_c(z);
        let sz = stumpff_s(z);
        if cz <= 0.0 {
            return -1.0;
        }
        r1_val + r2_val + (a_coeff * (z * sz - 1.0)) / cz.sqrt()
    };

    let calc_tof = |z: f64| -> f64 {
        let y = calc_y(z);
        if y <= 0.0 {
            return -1.0;
        }
        let cz = stumpff_c(z);
        let sz = stumpff_s(z);
        let chi = (y / cz).sqrt();
        (chi * chi * chi * sz + a_coeff * y.sqrt()) / mu_val.sqrt()
    };

    let t0 = calc_tof(0.0);
    let z_solution = if (t0 - tof_val).abs() < 1e-12 {
        0.0
    } else if tof_val > t0 {
        let z_low = 0.0;
        let mut z_high = 1.0;
        let max_z = 4.0 * PI * PI - 1e-4;
        while z_high < max_z {
            let y = calc_y(z_high);
            if y <= 0.0 {
                z_high = z_low + 0.5 * (z_high - z_low);
                break;
            }
            let tof = calc_tof(z_high);
            if tof >= tof_val {
                break;
            }
            z_high = (z_high * 1.5 + 0.5).min(max_z);
            if (z_high - max_z).abs() < 1e-6 {
                break;
            }
        }
        let mut f_obj = |z: f64| -> f64 { calc_tof(z) - tof_val };
        brent_root_find(&mut f_obj, z_low, z_high, 1e-12, 100).map_err(|_| {
            RocketDomainError::NumericalConvergence {
                context: "lambert_elliptic_root_find".to_string(),
                reason: "failed to converge in elliptic domain".to_string(),
            }
        })?
    } else {
        let z_high = 0.0;
        let mut z_low = -1.0;
        while z_low > -200.0 {
            let y = calc_y(z_low);
            if y <= 1e-6 {
                let mut y_l = z_low;
                let mut y_h = z_low * 0.5;
                for _ in 0..25 {
                    let mid = 0.5 * (y_l + y_h);
                    if calc_y(mid) > 1e-6 {
                        y_h = mid;
                    } else {
                        y_l = mid;
                    }
                }
                z_low = y_h;
                break;
            }
            let tof = calc_tof(z_low);
            if tof <= tof_val {
                break;
            }
            z_low *= 2.0;
        }
        let mut f_obj = |z: f64| -> f64 { calc_tof(z) - tof_val };
        brent_root_find(&mut f_obj, z_low, z_high, 1e-12, 100).map_err(|_| {
            RocketDomainError::NumericalConvergence {
                context: "lambert_hyperbolic_root_find".to_string(),
                reason: "failed to converge in hyperbolic domain".to_string(),
            }
        })?
    };

    let y_sol = calc_y(z_solution);
    if y_sol <= 0.0 || !y_sol.is_finite() {
        return Err(RocketDomainError::NumericalConvergence {
            context: "lambert_solution_verification".to_string(),
            reason: "computed y value is non-positive or non-finite".to_string(),
        });
    }

    let f = 1.0 - y_sol / r1_val;
    let g = a_coeff * (y_sol / mu_val).sqrt();
    let g_dot = 1.0 - y_sol / r2_val;

    if g.abs() < 1e-12 {
        return Err(RocketDomainError::NumericalConvergence {
            context: "lambert_lagrange_g".to_string(),
            reason: "Lagrange g coefficient is near zero".to_string(),
        });
    }

    let v1_vec = (r2_vec - r1_vec * f) / g;
    let v2_vec = (r2_vec * g_dot - r1_vec) / g;

    let v1_sq = v1_vec.dot(&v1_vec);
    let energy = 0.5 * v1_sq - mu_val / r1_val;
    let h_vec = r1_vec.cross(&v1_vec);

    let e_vec = v1_vec.cross(&h_vec) / mu_val - r1_vec / r1_val;
    let e = e_vec.magnitude();

    let sma = if energy.abs() < 1e-12 {
        Length::new(f64::INFINITY)
    } else {
        Length::new(-mu_val / (2.0 * energy))
    };

    let orbit_type = if (1.0 - e).abs() < 1e-6 || energy.abs() < 1e-12 {
        OrbitType::Parabolic
    } else if e < 1e-6 {
        OrbitType::Circular
    } else if e < 1.0 {
        OrbitType::Elliptic
    } else {
        OrbitType::Hyperbolic
    };

    Ok(LambertSolution {
        departure_velocity: VelocityVector::from_raw(v1_vec),
        arrival_velocity: VelocityVector::from_raw(v2_vec),
        semi_major_axis: sma,
        eccentricity: e,
        transfer_angle: Angle::new(delta_nu),
        orbit_type,
        time_of_flight,
        transfer_direction: direction,
    })
}