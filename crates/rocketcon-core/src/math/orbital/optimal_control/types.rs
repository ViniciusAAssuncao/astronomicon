use crate::error::{ RocketDomainError, RocketDomainResult };
use crate::math::orbital::low_thrust::ChebyshevTrajectoryApproximation;
use astronomicon_core::units::{
    Duration,
    Force,
    GravitationalParameter,
    Mass,
    Position,
    Speed,
    Vector3,
    VelocityVector,
};
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OptimalControlProblemType {
    MinimumFuel,
    MinimumTime,
    EnergyOptimal {
        smoothing_epsilon: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OptimalControlState {
    pub position: Vector3,
    pub velocity: Vector3,
    pub mass: f64,
    pub lambda_r: Vector3,
    pub lambda_v: Vector3,
    pub lambda_m: f64,
}

impl OptimalControlState {
    pub fn new(
        position: Vector3,
        velocity: Vector3,
        mass: f64,
        lambda_r: Vector3,
        lambda_v: Vector3,
        lambda_m: f64
    ) -> Self {
        Self {
            position,
            velocity,
            mass,
            lambda_r,
            lambda_v,
            lambda_m,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OptimalControlDerivative {
    pub d_position: Vector3,
    pub d_velocity: Vector3,
    pub d_mass: f64,
    pub d_lambda_r: Vector3,
    pub d_lambda_v: Vector3,
    pub d_lambda_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimalControlSolution {
    pub states: Vec<OptimalControlState>,
    pub times: Vec<Duration>,
    pub flight_time: Duration,
    pub final_mass: Mass,
    pub total_delta_v: Speed,
    pub propellant_consumed: Mass,
    pub hamiltonian_history: Vec<f64>,
}

impl OptimalControlSolution {
    pub fn to_chebyshev(
        &self,
        start_epoch: Duration,
        degree: usize
    ) -> RocketDomainResult<ChebyshevTrajectoryApproximation> {
        let n = self.states.len();
        if n < degree + 1 {
            return Err(RocketDomainError::InvalidInvariant {
                field: "states".to_string(),
                reason: "insufficient states to fit chebyshev polynomial".to_string(),
            });
        }

        let tf = self.flight_time.value();
        let mut tau_samples = Vec::with_capacity(n);
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        let mut zs = Vec::with_capacity(n);
        let mut vxs = Vec::with_capacity(n);
        let mut vys = Vec::with_capacity(n);
        let mut vzs = Vec::with_capacity(n);
        let mut masses = Vec::with_capacity(n);

        for (state, time) in self.states.iter().zip(self.times.iter()) {
            let t = time.value();
            let tau = if tf > 0.0 { (2.0 * t) / tf - 1.0 } else { 0.0 };
            tau_samples.push(tau.clamp(-1.0, 1.0));
            xs.push(state.position.0);
            ys.push(state.position.1);
            zs.push(state.position.2);
            vxs.push(state.velocity.0);
            vys.push(state.velocity.1);
            vzs.push(state.velocity.2);
            masses.push(state.mass);
        }

        let c_x = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &xs,
            degree
        )?;
        let c_y = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &ys,
            degree
        )?;
        let c_z = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &zs,
            degree
        )?;
        let c_vx = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &vxs,
            degree
        )?;
        let c_vy = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &vys,
            degree
        )?;
        let c_vz = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &vzs,
            degree
        )?;
        let c_mass = crate::math::orbital::low_thrust::fit_chebyshev_coefficients(
            &tau_samples,
            &masses,
            degree
        )?;

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShootingBoundaryConditions {
    pub initial_position: Position,
    pub initial_velocity: VelocityVector,
    pub initial_mass: Mass,
    pub target_position: Position,
    pub target_velocity: VelocityVector,
    pub time_of_flight: Duration,
    pub thrust: Force,
    pub specific_impulse: Duration,
    pub mu: GravitationalParameter,
    pub problem_type: OptimalControlProblemType,
}
