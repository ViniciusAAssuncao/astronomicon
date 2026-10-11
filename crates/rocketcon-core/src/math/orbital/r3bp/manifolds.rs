use super::types::{
    Cr3bpParameters, HaloOrbitState, ManifoldDirection, ManifoldTrajectory, ManifoldTube,
    ManifoldType, SynodicState,
};
use super::variational::{variational_rk4_step, Variational42State};
use crate::constants::{CR3BP_MANIFOLD_DEFAULT_STEP_COUNT, CR3BP_MANIFOLD_EPSILON_DEFAULT};
use crate::error::RocketDomainResult;
use astronomicon_core::units::Vector3;

pub fn extract_monodromy_eigenvectors(stm: &[[f64; 6]; 6]) -> (Vector3, Vector3) {
    let mut v_u = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
    for _ in 0..40 {
        let mut next_v = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                next_v[i] += stm[i][j] * v_u[j];
            }
        }
        let norm = next_v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
        for i in 0..6 {
            v_u[i] = next_v[i] / norm;
        }
    }

    let mut v_s = [1.0, 0.0, -1.0, 0.0, -1.0, 0.0];
    for _ in 0..40 {
        let mut next_v = [0.0; 6];
        for i in 0..6 {
            for j in 0..6 {
                next_v[i] += stm[j][i] * v_s[j];
            }
        }
        let norm = next_v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-12);
        for i in 0..6 {
            v_s[i] = next_v[i] / norm;
        }
    }

    (
        Vector3::new(v_u[0], v_u[1], v_u[2]).normalized(),
        Vector3::new(v_s[0], v_s[1], v_s[2]).normalized(),
    )
}

pub fn generate_invariant_manifold_trajectory(
    orbit_base_state: &SynodicState,
    eigenvector: Vector3,
    manifold_type: ManifoldType,
    direction: ManifoldDirection,
    mu: f64,
    epsilon: f64,
    propagation_duration_dimensionless: f64,
    step_count: usize,
) -> ManifoldTrajectory {
    let sign = match direction {
        ManifoldDirection::Exterior => 1.0,
        ManifoldDirection::Interior => -1.0,
    };

    let p_pert = orbit_base_state.position + eigenvector * (sign * epsilon);
    let v_pert = orbit_base_state.velocity;
    let init_pert = SynodicState::new(p_pert, v_pert);

    let total_t = propagation_duration_dimensionless;
    let dt_signed = match manifold_type {
        ManifoldType::Unstable => total_t / (step_count as f64),
        ManifoldType::Stable => -total_t / (step_count as f64),
    };

    let mut states = Vec::with_capacity(step_count + 1);
    let mut times = Vec::with_capacity(step_count + 1);

    let mut curr = init_pert;
    let mut t = 0.0;
    states.push(curr);
    times.push(t);

    for _ in 0..step_count {
        let deriv1 = super::equations::cr3bp_derivative(&curr, mu);
        let s2_p = curr.position + deriv1.position * (0.5 * dt_signed);
        let s2_v = curr.velocity + deriv1.velocity * (0.5 * dt_signed);
        let s2 = SynodicState::new(s2_p, s2_v);

        let deriv2 = super::equations::cr3bp_derivative(&s2, mu);
        let s3_p = curr.position + deriv2.position * (0.5 * dt_signed);
        let s3_v = curr.velocity + deriv2.velocity * (0.5 * dt_signed);
        let s3 = SynodicState::new(s3_p, s3_v);

        let deriv3 = super::equations::cr3bp_derivative(&s3, mu);
        let s4_p = curr.position + deriv3.position * dt_signed;
        let s4_v = curr.velocity + deriv3.velocity * dt_signed;
        let s4 = SynodicState::new(s4_p, s4_v);

        let deriv4 = super::equations::cr3bp_derivative(&s4, mu);

        let sixth = dt_signed / 6.0;
        curr.position = curr.position + (deriv1.position + deriv2.position * 2.0 + deriv3.position * 2.0 + deriv4.position) * sixth;
        curr.velocity = curr.velocity + (deriv1.velocity + deriv2.velocity * 2.0 + deriv3.velocity * 2.0 + deriv4.velocity) * sixth;
        t += dt_signed.abs();

        states.push(curr);
        times.push(t);
    }

    ManifoldTrajectory {
        manifold_type,
        direction,
        states,
        times_dimensionless: times,
    }
}

pub fn generate_invariant_manifold_tube(
    params: &Cr3bpParameters,
    halo_orbit: &HaloOrbitState,
    manifold_type: ManifoldType,
    direction: ManifoldDirection,
    tube_branches: usize,
    propagation_duration_dimensionless: f64,
) -> RocketDomainResult<ManifoldTube> {
    let mu = params.mu;
    let period = halo_orbit.period_dimensionless;
    let n_branches = tube_branches.max(4);

    let dt = period / 500.0;
    let mut var_state = Variational42State::new(halo_orbit.initial_state);
    let mut current_t = 0.0;

    while current_t < period {
        var_state = variational_rk4_step(&var_state, mu, dt);
        current_t += dt;
    }

    let monodromy = var_state.stm_matrix();
    let (v_u, v_s) = extract_monodromy_eigenvectors(&monodromy);
    let target_eigenvector = match manifold_type {
        ManifoldType::Unstable => v_u,
        ManifoldType::Stable => v_s,
    };

    let mut trajectories = Vec::with_capacity(n_branches);
    let mut orbit_marcher = Variational42State::new(halo_orbit.initial_state);
    let dt_branch = period / (n_branches as f64);

    for _ in 0..n_branches {
        let traj = generate_invariant_manifold_trajectory(
            &orbit_marcher.state,
            target_eigenvector,
            manifold_type,
            direction,
            mu,
            CR3BP_MANIFOLD_EPSILON_DEFAULT,
            propagation_duration_dimensionless,
            CR3BP_MANIFOLD_DEFAULT_STEP_COUNT,
        );
        trajectories.push(traj);

        let mut elapsed = 0.0;
        let march_dt = dt_branch / 20.0;
        while elapsed < dt_branch {
            orbit_marcher = variational_rk4_step(&orbit_marcher, mu, march_dt);
            elapsed += march_dt;
        }
    }

    Ok(ManifoldTube {
        manifold_type,
        direction,
        trajectories,
    })
}