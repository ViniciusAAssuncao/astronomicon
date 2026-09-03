use super::types::LowThrustPropagationResult;
use crate::domain::trajectory_patch::LowThrustPatchData;
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::{Duration, Force, Mass, Position, Speed, VelocityVector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChebyshevTrajectoryApproximation {
    pub start_epoch: Duration,
    pub end_epoch: Duration,
    pub degree: usize,
    pub c_x: Vec<f64>,
    pub c_y: Vec<f64>,
    pub c_z: Vec<f64>,
    pub c_vx: Vec<f64>,
    pub c_vy: Vec<f64>,
    pub c_vz: Vec<f64>,
    pub c_mass: Vec<f64>,
}

impl ChebyshevTrajectoryApproximation {
    pub fn evaluate_position(&self, epoch: Duration) -> Position {
        let tau = self.normalize_epoch(epoch);
        let x = evaluate_chebyshev_series(&self.c_x, tau);
        let y = evaluate_chebyshev_series(&self.c_y, tau);
        let z = evaluate_chebyshev_series(&self.c_z, tau);
        Position::from_components(x, y, z)
    }

    pub fn evaluate_velocity(&self, epoch: Duration) -> VelocityVector {
        let tau = self.normalize_epoch(epoch);
        let vx = evaluate_chebyshev_series(&self.c_vx, tau);
        let vy = evaluate_chebyshev_series(&self.c_vy, tau);
        let vz = evaluate_chebyshev_series(&self.c_vz, tau);
        VelocityVector::from_components(vx, vy, vz)
    }

    pub fn evaluate_mass(&self, epoch: Duration) -> Mass {
        let tau = self.normalize_epoch(epoch);
        let m = evaluate_chebyshev_series(&self.c_mass, tau);
        Mass::new(m.max(0.0))
    }

    pub fn evaluate(&self, epoch: Duration) -> (Position, VelocityVector, Mass) {
        (self.evaluate_position(epoch), self.evaluate_velocity(epoch), self.evaluate_mass(epoch))
    }

    pub fn to_patch_data(
        &self,
        thrust: Force,
        specific_impulse: Duration,
        total_delta_v: Speed,
    ) -> LowThrustPatchData {
        let initial_mass = self.evaluate_mass(self.start_epoch);
        let final_mass = self.evaluate_mass(self.end_epoch);
        LowThrustPatchData {
            initial_mass,
            final_mass,
            thrust,
            specific_impulse,
            total_delta_v,
            chebyshev_x: self.c_x.clone(),
            chebyshev_y: self.c_y.clone(),
            chebyshev_z: self.c_z.clone(),
            chebyshev_vx: self.c_vx.clone(),
            chebyshev_vy: self.c_vy.clone(),
            chebyshev_vz: self.c_vz.clone(),
            chebyshev_mass: self.c_mass.clone(),
        }
    }

    fn normalize_epoch(&self, epoch: Duration) -> f64 {
        let t = epoch.value();
        let t0 = self.start_epoch.value();
        let t1 = self.end_epoch.value();
        let dt = t1 - t0;
        if dt <= 1e-9 {
            return 0.0;
        }
        let tau = (2.0 * (t - t0)) / dt - 1.0;
        tau.clamp(-1.0, 1.0)
    }
}

pub fn evaluate_chebyshev_series(coeffs: &[f64], tau: f64) -> f64 {
    let n = coeffs.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return coeffs[0];
    }
    let tau2 = 2.0 * tau;
    let mut b_kplus1 = 0.0;
    let mut b_kplus2 = 0.0;
    for i in (1..n).rev() {
        let b_k = tau2 * b_kplus1 - b_kplus2 + coeffs[i];
        b_kplus2 = b_kplus1;
        b_kplus1 = b_k;
    }
    tau * b_kplus1 - b_kplus2 + coeffs[0]
}

pub fn fit_chebyshev_coefficients(
    tau_samples: &[f64],
    values: &[f64],
    degree: usize,
) -> RocketDomainResult<Vec<f64>> {
    let m = tau_samples.len();
    if m < degree + 1 || values.len() != m {
        return Err(RocketDomainError::InvalidInvariant {
            field: "chebyshev_samples".to_string(),
            reason: "number of samples must be greater than or equal to degree + 1".to_string(),
        });
    }

    let p = degree + 1;
    let mut basis = vec![vec![0.0; p]; m];

    for i in 0..m {
        let tau = tau_samples[i].clamp(-1.0, 1.0);
        basis[i][0] = 1.0;
        if p > 1 {
            basis[i][1] = tau;
            for k in 2..p {
                basis[i][k] = 2.0 * tau * basis[i][k - 1] - basis[i][k - 2];
            }
        }
    }

    let mut ata = vec![vec![0.0; p]; p];
    let mut aty = vec![0.0; p];

    for i in 0..m {
        let y = values[i];
        for j in 0..p {
            let bj = basis[i][j];
            aty[j] += bj * y;
            for k in 0..p {
                ata[j][k] += bj * basis[i][k];
            }
        }
    }

    for j in 0..p {
        ata[j][j] += 1e-12;
    }

    let mut l = vec![vec![0.0; p]; p];
    for i in 0..p {
        for j in 0..=i {
            let mut sum = ata[i][j];
            for k in 0..j {
                sum -= l[i][k] * l[j][k];
            }
            if i == j {
                if sum <= 0.0 {
                    return Err(RocketDomainError::NumericalConvergence {
                        context: "chebyshev_cholesky".to_string(),
                        reason: "matrix is not positive definite".to_string(),
                    });
                }
                l[i][j] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }

    let mut z = vec![0.0; p];
    for i in 0..p {
        let mut sum = aty[i];
        for k in 0..i {
            sum -= l[i][k] * z[k];
        }
        z[i] = sum / l[i][i];
    }

    let mut coeffs = vec![0.0; p];
    for i in (0..p).rev() {
        let mut sum = z[i];
        for k in i + 1..p {
            sum -= l[k][i] * coeffs[k];
        }
        coeffs[i] = sum / l[i][i];
    }

    Ok(coeffs)
}

impl LowThrustPropagationResult {
    pub fn fit_chebyshev(
        &self,
        degree: usize,
        start_epoch: Duration,
    ) -> RocketDomainResult<ChebyshevTrajectoryApproximation> {
        let n = self.states.len();
        if n < degree + 1 {
            return Err(RocketDomainError::InvalidInvariant {
                field: "states".to_string(),
                reason: "insufficient trajectory states for requested degree".to_string(),
            });
        }

        let total_time = self.flight_time.value();
        let mut tau_samples = Vec::with_capacity(n);
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        let mut zs = Vec::with_capacity(n);
        let mut vxs = Vec::with_capacity(n);
        let mut vys = Vec::with_capacity(n);
        let mut vzs = Vec::with_capacity(n);
        let mut masses = Vec::with_capacity(n);

        for state in &self.states {
            let t = state.time.value();
            let tau = if total_time > 0.0 { (2.0 * t) / total_time - 1.0 } else { 0.0 };
            tau_samples.push(tau.clamp(-1.0, 1.0));

            let p = state.position.raw();
            let v = state.velocity.raw();
            xs.push(p.0);
            ys.push(p.1);
            zs.push(p.2);
            vxs.push(v.0);
            vys.push(v.1);
            vzs.push(v.2);
            masses.push(state.mass.value());
        }

        let c_x = fit_chebyshev_coefficients(&tau_samples, &xs, degree)?;
        let c_y = fit_chebyshev_coefficients(&tau_samples, &ys, degree)?;
        let c_z = fit_chebyshev_coefficients(&tau_samples, &zs, degree)?;
        let c_vx = fit_chebyshev_coefficients(&tau_samples, &vxs, degree)?;
        let c_vy = fit_chebyshev_coefficients(&tau_samples, &vys, degree)?;
        let c_vz = fit_chebyshev_coefficients(&tau_samples, &vzs, degree)?;
        let c_mass = fit_chebyshev_coefficients(&tau_samples, &masses, degree)?;

        Ok(ChebyshevTrajectoryApproximation {
            start_epoch,
            end_epoch: start_epoch + self.flight_time,
            degree,
            c_x,
            c_y,
            c_z,
            c_vx,
            c_vy,
            c_vz,
            c_mass,
        })
    }
}