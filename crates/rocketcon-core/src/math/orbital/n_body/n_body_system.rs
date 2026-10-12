use super::cowell::cowell_acceleration_vector;
use super::integrator_dp853::{adaptive_dp853_step, Dp853State};
use super::types::{NBodyPropagationConfig, NBodyPropagationResult, NBodyTrajectoryPoint};
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::{Duration, Position, Speed, VelocityVector};

pub fn compute_system_specific_energy(
    pos: Position,
    vel: VelocityVector,
    mu: f64,
) -> f64 {
    let r = pos.raw().magnitude();
    let v_sq = vel.raw().dot(&vel.raw());
    if r > 0.0 {
        0.5 * v_sq - mu / r
    } else {
        0.0
    }
}

pub fn propagate_n_body(
    config: &NBodyPropagationConfig,
) -> RocketDomainResult<NBodyPropagationResult> {
    let total_duration = config.duration.value();
    let mu = config.cowell_config.primary_body.gravitational_parameter().value();

    if total_duration <= 0.0 || mu <= 0.0 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "propagation_config".to_string(),
            reason: "duration and central mass parameter must be strictly positive".to_string(),
        });
    }

    let mut t = 0.0;
    let mut dt = config.initial_step.value().max(1.0);
    let mut curr_state = Dp853State::new(
        config.initial_position.raw(),
        config.initial_velocity.raw(),
    );

    let e_init = compute_system_specific_energy(
        config.initial_position,
        config.initial_velocity,
        mu,
    );

    let mut points = Vec::with_capacity(500);
    let init_acc = cowell_acceleration_vector(curr_state.position, &config.cowell_config);

    points.push(NBodyTrajectoryPoint {
        time: config.start_epoch,
        position: config.initial_position,
        velocity: config.initial_velocity,
        acceleration: init_acc,
        specific_energy: e_init,
    });

    let mut accumulated_dv = 0.0;
    let mut prev_v = curr_state.velocity;

    while t < total_duration {
        let step = dt.min(total_duration - t);
        let (next_state, next_dt, accepted) = adaptive_dp853_step(
            &curr_state,
            &config.cowell_config,
            step,
            config.absolute_tolerance,
            config.relative_tolerance,
        );

        if accepted {
            let dv_step = (next_state.velocity - prev_v).magnitude();
            accumulated_dv += dv_step;
            prev_v = next_state.velocity;

            curr_state = next_state;
            t += step;

            let pos = Position::from_raw(curr_state.position);
            let vel = VelocityVector::from_raw(curr_state.velocity);
            let acc = cowell_acceleration_vector(curr_state.position, &config.cowell_config);
            let energy = compute_system_specific_energy(pos, vel, mu);

            points.push(NBodyTrajectoryPoint {
                time: config.start_epoch + Duration::new(t),
                position: pos,
                velocity: vel,
                acceleration: acc,
                specific_energy: energy,
            });
        }

        dt = next_dt;
    }

    let e_final = points.last().map(|p| p.specific_energy).unwrap_or(e_init);
    let energy_drift = if e_init.abs() > 1e-6 {
        ((e_final - e_init) / e_init).abs()
    } else {
        (e_final - e_init).abs()
    };

    Ok(NBodyPropagationResult {
        points,
        final_epoch: config.start_epoch + Duration::new(t),
        initial_specific_energy: e_init,
        final_specific_energy: e_final,
        energy_drift_fraction: energy_drift,
        total_delta_v_absorbed: Speed::new(accumulated_dv),
    })
}