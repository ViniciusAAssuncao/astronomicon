use super::types::{Cr3bpParameters, HaloOrbitFamily, LibrationPoint, SynodicState};
use crate::constants::{CR3BP_COLLINEAR_CONVERGENCE_TOLERANCE, CR3BP_MAX_NEWTON_ITERATIONS};
use astronomicon_core::units::Vector3;
use std::f64::consts::PI;

pub fn solve_collinear_gamma(mu: f64, point: LibrationPoint) -> f64 {
    let mut gamma = (mu / 3.0).cbrt().max(1e-4);
    for _ in 0..CR3BP_MAX_NEWTON_ITERATIONS {
        let g2 = gamma * gamma;
        let g3 = g2 * gamma;
        let g4 = g3 * gamma;
        let g5 = g4 * gamma;

        let (f, df) = match point {
            LibrationPoint::L1 => {
                let val = g5 - (3.0 - mu) * g4 + (3.0 - 2.0 * mu) * g3 - mu * g2 + 2.0 * mu * gamma - mu;
                let d_val = 5.0 * g4 - 4.0 * (3.0 - mu) * g3 + 3.0 * (3.0 - 2.0 * mu) * g2 - 2.0 * mu * gamma + 2.0 * mu;
                (val, d_val)
            }
            LibrationPoint::L2 => {
                let val = g5 + (3.0 - mu) * g4 + (3.0 - 2.0 * mu) * g3 - mu * g2 - 2.0 * mu * gamma - mu;
                let d_val = 5.0 * g4 + 4.0 * (3.0 - mu) * g3 + 3.0 * (3.0 - 2.0 * mu) * g2 - 2.0 * mu * gamma - 2.0 * mu;
                (val, d_val)
            }
            _ => (0.0, 1.0),
        };

        if df.abs() < 1e-15 {
            break;
        }

        let delta = f / df;
        gamma -= delta;
        if delta.abs() < CR3BP_COLLINEAR_CONVERGENCE_TOLERANCE {
            break;
        }
    }
    gamma.abs()
}

pub fn collinear_location_x(mu: f64, point: LibrationPoint) -> f64 {
    let gamma = solve_collinear_gamma(mu, point);
    match point {
        LibrationPoint::L1 => 1.0 - mu - gamma,
        LibrationPoint::L2 => 1.0 - mu + gamma,
        LibrationPoint::L3 => -mu - 1.0 - (5.0 * mu) / 12.0,
        LibrationPoint::L4 | LibrationPoint::L5 => 0.5 - mu,
    }
}

pub fn richardson_halo_approximation(
    params: &Cr3bpParameters,
    point: LibrationPoint,
    family: HaloOrbitFamily,
    az_amplitude: f64,
) -> (SynodicState, f64) {
    let mu = params.mu;
    let gamma = solve_collinear_gamma(mu, point);
    let x_l = collinear_location_x(mu, point);

    let c2 = match point {
        LibrationPoint::L1 => (1.0 / gamma.powi(3)) * (mu + (1.0 - mu) * gamma.powi(3) / (1.0 - gamma).powi(3)),
        LibrationPoint::L2 => (1.0 / gamma.powi(3)) * (mu + (1.0 - mu) * gamma.powi(3) / (1.0 + gamma).powi(3)),
        _ => 2.0,
    };

    let c3 = match point {
        LibrationPoint::L1 => (1.0 / gamma.powi(3)) * (mu - (1.0 - mu) * gamma.powi(4) / (1.0 - gamma).powi(4)),
        LibrationPoint::L2 => (1.0 / gamma.powi(3)) * (-mu - (1.0 - mu) * gamma.powi(4) / (1.0 + gamma).powi(4)),
        _ => 0.0,
    };

    let disc = ((4.0 - c2) * (4.0 - c2) + 4.0 * (2.0 * c2 - 1.0) * (c2 + 1.0)).max(0.0);
    let lambda_sq = (c2 - 4.0 + disc.sqrt()) * 0.5;
    let lambda = lambda_sq.max(1e-6).sqrt();
    let k = (2.0 * lambda) / (lambda_sq + 1.0 - c2).max(1e-6);

    let a21 = (3.0 * c3 * (k * k - 2.0)) / (4.0 * (1.0 + 2.0 * c2)).max(1e-6);
    let a22 = (3.0 * c3 * k * k) / (4.0 * (1.0 + 2.0 * c2)).max(1e-6);

    let l1 = a21 + 2.0 * a22;
    let l2 = a21 - 2.0 * a22;

    let az = az_amplitude / gamma;
    let ax = if l2.abs() > 1e-12 {
        ((-l1 * az * az) / l2).max(0.0).sqrt()
    } else {
        az * 0.5
    };

    let sign_z = match family {
        HaloOrbitFamily::Northern => 1.0,
        HaloOrbitFamily::Southern => -1.0,
    };

    let delta_x = gamma * (-ax + a21 * ax * ax + a22 * az * az);
    let delta_y = 0.0;
    let delta_z = gamma * sign_z * az;

    let vy = gamma * lambda * k * ax;

    let x0 = x_l + delta_x;
    let y0 = delta_y;
    let z0 = delta_z;

    let vx0 = 0.0;
    let vy0 = vy;
    let vz0 = 0.0;

    let period = (2.0 * PI) / lambda;

    (
        SynodicState::new(Vector3::new(x0, y0, z0), Vector3::new(vx0, vy0, vz0)),
        period,
    )
}