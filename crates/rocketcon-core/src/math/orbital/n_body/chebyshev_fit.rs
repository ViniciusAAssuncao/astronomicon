use super::types::NBodyPropagationResult;
use crate::domain::trajectory_patch::LowThrustPatchData;
use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::low_thrust::fit_chebyshev_coefficients;
use astronomicon_core::units::{Duration, Force, Mass};

pub fn n_body_trajectory_to_chebyshev_patch(
    result: &NBodyPropagationResult,
    degree: usize,
    mass: Mass,
) -> RocketDomainResult<LowThrustPatchData> {
    let n = result.points.len();
    if n < degree + 1 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "points".to_string(),
            reason: "insufficient N-body trajectory points to fit Chebyshev polynomial".to_string(),
        });
    }

    let t0 = result.points.first().map(|p| p.time.value()).unwrap_or(0.0);
    let t1 = result.points.last().map(|p| p.time.value()).unwrap_or(t0 + 1.0);
    let total_time = (t1 - t0).max(1e-4);

    let mut tau_samples = Vec::with_capacity(n);
    let mut xs = Vec::with_capacity(n);
    let mut ys = Vec::with_capacity(n);
    let mut zs = Vec::with_capacity(n);
    let mut vxs = Vec::with_capacity(n);
    let mut vys = Vec::with_capacity(n);
    let mut vzs = Vec::with_capacity(n);
    let mut masses = Vec::with_capacity(n);

    for pt in &result.points {
        let t = pt.time.value();
        let tau = (2.0 * (t - t0)) / total_time - 1.0;
        tau_samples.push(tau.clamp(-1.0, 1.0));

        let p = pt.position.raw();
        let v = pt.velocity.raw();
        xs.push(p.0);
        ys.push(p.1);
        zs.push(p.2);
        vxs.push(v.0);
        vys.push(v.1);
        vzs.push(v.2);
        masses.push(mass.value());
    }

    let c_x = fit_chebyshev_coefficients(&tau_samples, &xs, degree)?;
    let c_y = fit_chebyshev_coefficients(&tau_samples, &ys, degree)?;
    let c_z = fit_chebyshev_coefficients(&tau_samples, &zs, degree)?;
    let c_vx = fit_chebyshev_coefficients(&tau_samples, &vxs, degree)?;
    let c_vy = fit_chebyshev_coefficients(&tau_samples, &vys, degree)?;
    let c_vz = fit_chebyshev_coefficients(&tau_samples, &vzs, degree)?;
    let c_mass = fit_chebyshev_coefficients(&tau_samples, &masses, degree)?;

    Ok(LowThrustPatchData {
        initial_mass: mass,
        final_mass: mass,
        thrust: Force::new(0.0),
        specific_impulse: Duration::new(1.0),
        total_delta_v: result.total_delta_v_absorbed,
        chebyshev_x: c_x,
        chebyshev_y: c_y,
        chebyshev_z: c_z,
        chebyshev_vx: c_vx,
        chebyshev_vy: c_vy,
        chebyshev_vz: c_vz,
        chebyshev_mass: c_mass,
    })
}