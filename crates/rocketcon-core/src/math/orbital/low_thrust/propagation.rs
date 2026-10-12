use super::types::{
    LowThrustPropagationConfig, LowThrustPropagationResult, LowThrustSteeringMode,
    LowThrustTrajectoryState,
};
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{Duration, Mass, Position, Speed, Vector3, VelocityVector};

fn compute_thrust_direction(
    pos: Vector3,
    vel: Vector3,
    steering: LowThrustSteeringMode,
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
    steering: LowThrustSteeringMode,
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
    config: &LowThrustPropagationConfig,
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

    states.push(LowThrustTrajectoryState::new(
        Duration::new(current_time),
        Position::from_raw(pos),
        VelocityVector::from_raw(vel),
        Mass::new(mass),
        Speed::new(accumulated_dv),
    ));

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
            config.steering_mode,
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
            config.steering_mode,
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
            config.steering_mode,
        );

        let sixth_dt = dt / 6.0;
        pos = pos + (v1 + v2 * 2.0 + v3 * 2.0 + v4) * sixth_dt;
        vel = vel + (a1 + a2 * 2.0 + a3 * 2.0 + a4) * sixth_dt;
        mass += (dm1 + dm2 * 2.0 + dm3 * 2.0 + dm4) * sixth_dt;

        let step_accel = f / mass;
        accumulated_dv += step_accel * dt;
        current_time += dt;

        states.push(LowThrustTrajectoryState::new(
            Duration::new(current_time),
            Position::from_raw(pos),
            VelocityVector::from_raw(vel),
            Mass::new(mass),
            Speed::new(accumulated_dv),
        ));
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