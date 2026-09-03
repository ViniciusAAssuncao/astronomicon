use crate::domain::trajectory_patch::LowThrustPatchData;
use crate::error::{ RocketDomainError, RocketDomainResult };
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{
    Angle,
    Duration,
    Force,
    GravitationalParameter,
    Length,
    Mass,
    MassRate,
    Position,
    Speed,
    Vector3,
    VelocityVector,
};
use serde::{ Deserialize, Serialize };
use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LowThrustSteeringMode {
    Tangential,
    Inertial(Vector3),
    CircularityOptimal,
    OptimalPrimerVector(Vector3),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LowThrustTrajectoryState {
    pub time: Duration,
    pub position: Position,
    pub velocity: VelocityVector,
    pub mass: Mass,
    pub accumulated_delta_v: Speed,
}

impl LowThrustTrajectoryState {
    pub fn new(
        time: Duration,
        position: Position,
        velocity: VelocityVector,
        mass: Mass,
        accumulated_delta_v: Speed
    ) -> Self {
        Self {
            time,
            position,
            velocity,
            mass,
            accumulated_delta_v,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowThrustPropagationConfig {
    pub initial_position: Position,
    pub initial_velocity: VelocityVector,
    pub initial_mass: Mass,
    pub thrust: Force,
    pub specific_impulse: Duration,
    pub mu: GravitationalParameter,
    pub steering_mode: LowThrustSteeringMode,
    pub duration: Duration,
    pub time_step: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LowThrustPropagationResult {
    pub states: Vec<LowThrustTrajectoryState>,
    pub final_mass: Mass,
    pub total_delta_v: Speed,
    pub total_propellant_consumed: Mass,
    pub flight_time: Duration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdelbaumTransferResult {
    pub total_delta_v: Speed,
    pub flight_time: Duration,
    pub initial_mass: Mass,
    pub final_mass: Mass,
    pub propellant_consumed: Mass,
    pub mass_flow_rate: MassRate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpiralEscapeResult {
    pub initial_radius: Length,
    pub target_radius: Length,
    pub total_delta_v: Speed,
    pub flight_time: Duration,
    pub final_mass: Mass,
    pub propellant_consumed: Mass,
    pub revolutions_estimate: f64,
}

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
        total_delta_v: Speed
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
    degree: usize
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
        start_epoch: Duration
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

pub fn edelbaum_low_thrust_transfer(
    r_initial: Length,
    r_final: Length,
    inclination_change: Angle,
    initial_mass: Mass,
    thrust: Force,
    specific_impulse: Duration,
    mu: GravitationalParameter
) -> RocketDomainResult<EdelbaumTransferResult> {
    let r0 = r_initial.value();
    let rf = r_final.value();
    let di = inclination_change.value().abs();
    let m0 = initial_mass.value();
    let f = thrust.value();
    let isp = specific_impulse.value();
    let mu_val = mu.value();

    if
        r0 <= 0.0 ||
        rf <= 0.0 ||
        m0 <= 0.0 ||
        f <= 0.0 ||
        isp <= 0.0 ||
        mu_val <= 0.0 ||
        !r0.is_finite() ||
        !rf.is_finite() ||
        !m0.is_finite() ||
        !f.is_finite() ||
        !isp.is_finite() ||
        !mu_val.is_finite()
    {
        return Err(RocketDomainError::InvalidInvariant {
            field: "edelbaum_parameters".to_string(),
            reason: "parameters must be positive and finite".to_string(),
        });
    }

    let v0 = (mu_val / r0).sqrt();
    let vf = (mu_val / rf).sqrt();
    let cos_term = (0.5 * PI * di).cos();
    let dv_sq = v0 * v0 - 2.0 * v0 * vf * cos_term + vf * vf;
    let delta_v = dv_sq.max(0.0).sqrt();

    let ve = isp * STANDARD_GRAVITY;
    let m_dot = f / ve;
    let mass_ratio = (-delta_v / ve).exp();
    let mf = m0 * mass_ratio;
    let prop_consumed = m0 - mf;
    let flight_time = if m_dot > 0.0 { prop_consumed / m_dot } else { 0.0 };

    Ok(EdelbaumTransferResult {
        total_delta_v: Speed::new(delta_v),
        flight_time: Duration::new(flight_time),
        initial_mass,
        final_mass: Mass::new(mf),
        propellant_consumed: Mass::new(prop_consumed),
        mass_flow_rate: MassRate::new(m_dot),
    })
}

pub fn logarithmic_spiral_escape(
    r_initial: Length,
    r_target: Length,
    initial_mass: Mass,
    thrust: Force,
    specific_impulse: Duration,
    mu: GravitationalParameter
) -> RocketDomainResult<SpiralEscapeResult> {
    let r0 = r_initial.value();
    let rf = r_target.value();
    let m0 = initial_mass.value();
    let f = thrust.value();
    let isp = specific_impulse.value();
    let mu_val = mu.value();

    if
        r0 <= 0.0 ||
        rf <= r0 ||
        m0 <= 0.0 ||
        f <= 0.0 ||
        isp <= 0.0 ||
        mu_val <= 0.0 ||
        !r0.is_finite() ||
        !rf.is_finite() ||
        !m0.is_finite() ||
        !f.is_finite() ||
        !isp.is_finite() ||
        !mu_val.is_finite()
    {
        return Err(RocketDomainError::InvalidInvariant {
            field: "spiral_parameters".to_string(),
            reason: "parameters must be positive, finite, and r_target > r_initial".to_string(),
        });
    }

    let v0 = (mu_val / r0).sqrt();
    let vf = (mu_val / rf).sqrt();
    let delta_v = (v0 - vf).abs();

    let ve = isp * STANDARD_GRAVITY;
    let m_dot = f / ve;
    let mass_ratio = (-delta_v / ve).exp();
    let mf = m0 * mass_ratio;
    let prop_consumed = m0 - mf;
    let flight_time = if m_dot > 0.0 { prop_consumed / m_dot } else { 0.0 };

    let mean_accel = f / (0.5 * (m0 + mf));
    let revs = if mean_accel > 0.0 {
        (mu_val / (4.0 * PI * mean_accel)) * (1.0 / r0 - 1.0 / rf)
    } else {
        0.0
    };

    Ok(SpiralEscapeResult {
        initial_radius: r_initial,
        target_radius: r_target,
        total_delta_v: Speed::new(delta_v),
        flight_time: Duration::new(flight_time),
        final_mass: Mass::new(mf),
        propellant_consumed: Mass::new(prop_consumed),
        revolutions_estimate: revs.max(0.0),
    })
}

fn compute_thrust_direction(
    pos: Vector3,
    vel: Vector3,
    steering: LowThrustSteeringMode
) -> Vector3 {
    match steering {
        LowThrustSteeringMode::Tangential => {
            let v_mag = vel.magnitude();
            if v_mag > 1e-12 {
                vel / v_mag
            } else {
                Vector3::new(1.0, 0.0, 0.0)
            }
        }
        LowThrustSteeringMode::Inertial(dir) => dir.normalized(),
        LowThrustSteeringMode::CircularityOptimal => {
            let r_mag = pos.magnitude();
            let v_mag = vel.magnitude();
            if r_mag > 1e-12 && v_mag > 1e-12 {
                let u_r = pos / r_mag;
                let u_v = vel / v_mag;
                let flight_path_sin = u_r.dot(&u_v);
                let u_transverse = (vel - u_r * (v_mag * flight_path_sin)).normalized();
                (u_transverse - u_r * flight_path_sin).normalized()
            } else {
                Vector3::new(1.0, 0.0, 0.0)
            }
        }
        LowThrustSteeringMode::OptimalPrimerVector(p) => {
            let p_mag = p.magnitude();
            if p_mag > 1e-12 {
                p / p_mag
            } else {
                Vector3::new(1.0, 0.0, 0.0)
            }
        }
    }
}

fn low_thrust_dynamics(
    pos: Vector3,
    vel: Vector3,
    mass: f64,
    mu: f64,
    thrust: f64,
    m_dot: f64,
    steering: LowThrustSteeringMode
) -> (Vector3, Vector3, f64) {
    let r_mag = pos.magnitude();
    if r_mag < 1e-6 || mass <= 0.0 {
        return (vel, Vector3::zero(), 0.0);
    }

    let a_grav = -pos * (mu / (r_mag * r_mag * r_mag));
    let u_thrust = compute_thrust_direction(pos, vel, steering);
    let a_thrust = u_thrust * (thrust / mass);
    let a_total = a_grav + a_thrust;

    (vel, a_total, -m_dot)
}

pub fn propagate_low_thrust(
    config: &LowThrustPropagationConfig
) -> RocketDomainResult<LowThrustPropagationResult> {
    let total_time = config.duration.value();
    let dt_target = config.time_step.value().clamp(1.0, 3600.0);
    let mu = config.mu.value();
    let f = config.thrust.value();
    let isp = config.specific_impulse.value();

    if total_time <= 0.0 || mu <= 0.0 || f <= 0.0 || isp <= 0.0 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "low_thrust_config".to_string(),
            reason: "propagation parameters must be strictly positive".to_string(),
        });
    }

    let ve = isp * STANDARD_GRAVITY;
    let m_dot = f / ve;

    let steps = ((total_time / dt_target).ceil() as usize).max(2);
    let dt = total_time / (steps as f64);

    let mut states = Vec::with_capacity(steps + 1);
    let mut pos = config.initial_position.raw();
    let mut vel = config.initial_velocity.raw();
    let mut mass = config.initial_mass.value();
    let mut accumulated_dv = 0.0;
    let mut current_time = 0.0;

    states.push(
        LowThrustTrajectoryState::new(
            Duration::new(current_time),
            Position::from_raw(pos),
            VelocityVector::from_raw(vel),
            Mass::new(mass),
            Speed::new(accumulated_dv)
        )
    );

    for _ in 0..steps {
        if mass <= 1.0 {
            break;
        }

        let half_dt = 0.5 * dt;

        let (v1, a1, dm1) = low_thrust_dynamics(pos, vel, mass, mu, f, m_dot, config.steering_mode);

        let p2 = pos + v1 * half_dt;
        let v2_step = vel + a1 * half_dt;
        let m2 = mass + dm1 * half_dt;
        let (v2, a2, dm2) = low_thrust_dynamics(
            p2,
            v2_step,
            m2,
            mu,
            f,
            m_dot,
            config.steering_mode
        );

        let p3 = pos + v2 * half_dt;
        let v3_step = vel + a2 * half_dt;
        let m3 = mass + dm2 * half_dt;
        let (v3, a3, dm3) = low_thrust_dynamics(
            p3,
            v3_step,
            m3,
            mu,
            f,
            m_dot,
            config.steering_mode
        );

        let p4 = pos + v3 * dt;
        let v4_step = vel + a3 * dt;
        let m4 = mass + dm3 * dt;
        let (v4, a4, dm4) = low_thrust_dynamics(
            p4,
            v4_step,
            m4,
            mu,
            f,
            m_dot,
            config.steering_mode
        );

        let sixth_dt = dt / 6.0;
        pos = pos + (v1 + v2 * 2.0 + v3 * 2.0 + v4) * sixth_dt;
        vel = vel + (a1 + a2 * 2.0 + a3 * 2.0 + a4) * sixth_dt;
        mass += (dm1 + dm2 * 2.0 + dm3 * 2.0 + dm4) * sixth_dt;

        let step_accel = f / mass;
        accumulated_dv += step_accel * dt;
        current_time += dt;

        states.push(
            LowThrustTrajectoryState::new(
                Duration::new(current_time),
                Position::from_raw(pos),
                VelocityVector::from_raw(vel),
                Mass::new(mass),
                Speed::new(accumulated_dv)
            )
        );
    }

    let initial_m = config.initial_mass.value();
    let prop_consumed = (initial_m - mass).max(0.0);

    Ok(LowThrustPropagationResult {
        states,
        final_mass: Mass::new(mass),
        total_delta_v: Speed::new(accumulated_dv),
        total_propellant_consumed: Mass::new(prop_consumed),
        flight_time: Duration::new(current_time),
    })
}
