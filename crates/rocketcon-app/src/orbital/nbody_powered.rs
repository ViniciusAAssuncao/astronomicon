use super::nbody_field::tick_policy;
use crate::error::{RocketError, RocketResult};
use astronomicon_core::units::{
    AngularVelocityVector, Duration, Position, Quaternion, Vector3, VelocityVector,
};
use rocketcon_core::math::orbital::n_body::dop853::{Dop853Error, solve_dop853};
use rocketcon_core::math::{RigidBodyDerivative, RigidBodyState};

fn stage_state(y: &[f64; 13]) -> RigidBodyState {
    RigidBodyState::new(
        Position::from_raw(Vector3::new(y[0], y[1], y[2])),
        VelocityVector::from_raw(Vector3::new(y[3], y[4], y[5])),
        Quaternion::new(y[6], y[7], y[8], y[9]),
        AngularVelocityVector::from_raw(Vector3::new(y[10], y[11], y[12])),
    )
}

fn stage_derivative(state: &RigidBodyState, derivative: RigidBodyDerivative) -> [f64; 13] {
    let pos = derivative.velocity.raw();
    let vel = derivative.acceleration.raw();
    let quaternion = state.orientation.derivative(derivative.angular_velocity);
    let omega = derivative.angular_acceleration.raw();
    [
        pos.0,
        pos.1,
        pos.2,
        vel.0,
        vel.1,
        vel.2,
        quaternion.w(),
        quaternion.x(),
        quaternion.y(),
        quaternion.z(),
        omega.0,
        omega.1,
        omega.2,
    ]
}

pub(crate) fn integrate_nbody_rigid_body<F>(
    initial: &RigidBodyState,
    dt: Duration,
    mut evaluate: F,
) -> RocketResult<RigidBodyState>
where
    F: FnMut(f64, &RigidBodyState) -> Result<RigidBodyDerivative, Dop853Error>,
{
    let origin = initial.position.raw();
    let v = initial.velocity.raw();
    let q = initial.orientation;
    let w = initial.angular_velocity.raw();
    let y = [
        0.0,
        0.0,
        0.0,
        v.0,
        v.1,
        v.2,
        q.w(),
        q.x(),
        q.y(),
        q.z(),
        w.0,
        w.1,
        w.2,
    ];
    if !y.iter().all(|value| value.is_finite()) {
        return Err(RocketError::Generic(
            "nonfinite initial rigid-body state".into(),
        ));
    }
    let configuration = tick_policy(
        dt,
        [
            1e-3, 1e-3, 1e-3, 1e-5, 1e-5, 1e-5, 1e-9, 1e-9, 1e-9, 1e-9, 1e-7, 1e-7, 1e-7,
        ],
    )?;
    let solved = solve_dop853(
        y,
        dt.value(),
        configuration,
        |t, y| {
            let sub_state = stage_state(y);
            let derivative = evaluate(t, &sub_state)?;
            Ok(stage_derivative(&sub_state, derivative))
        },
        || false,
    )
    .map_err(|error| {
        RocketError::Generic(format!("powered N-body integration failed: {error:?}"))
    })?;
    if solved.event_seconds.is_some() || solved.reached_seconds != dt.value() {
        return Err(RocketError::Generic(
            "powered N-body did not complete the authoritative tick".into(),
        ));
    }
    let result = stage_state(&solved.final_state);
    let quaternion = result.orientation;
    let norm = quaternion.norm();
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err(RocketError::Generic(
            "invalid integrated orientation".into(),
        ));
    }
    Ok(RigidBodyState::new(
        Position::from_raw(origin + result.position.raw()),
        result.velocity,
        quaternion.normalized(),
        result.angular_velocity,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::{
        AccelerationVector, AngularAccelerationVector, AngularVelocityVector,
    };

    #[test]
    fn adaptive_rigid_body_step_preserves_thrust_acceleration_and_attitude() {
        let state = RigidBodyState::new(
            Position::from_raw(Vector3::new(1e8, 2e8, 0.0)),
            VelocityVector::from_raw(Vector3::new(15.0, -8.0, 0.0)),
            Quaternion::identity(),
            AngularVelocityVector::zero(),
        );
        let duration = Duration::new(0.2);
        let result = integrate_nbody_rigid_body(&state, duration, |_, sub| {
            Ok(RigidBodyDerivative::new(
                sub.velocity,
                AccelerationVector::from_raw(Vector3::new(4.0, 6.0, 0.0)),
                sub.angular_velocity,
                AngularAccelerationVector::zero(),
            ))
        })
        .unwrap();
        let delta = result.position.raw() - state.position.raw();
        assert!((delta.0 - 3.08).abs() < 1e-6);
        assert!((delta.1 - (-1.48)).abs() < 1e-6);
        assert!((result.velocity.raw().0 - 15.8).abs() < 1e-9);
        assert!((result.velocity.raw().1 - (-6.8)).abs() < 1e-9);
        assert!(result.orientation.is_normalized(1e-12));
    }

    #[test]
    fn adaptive_rigid_body_step_integrates_angular_acceleration() {
        let initial = RigidBodyState::new(
            Position::zero(),
            VelocityVector::zero(),
            Quaternion::identity(),
            AngularVelocityVector::zero(),
        );
        let result = integrate_nbody_rigid_body(&initial, Duration::new(0.1), |_, sub| {
            Ok(RigidBodyDerivative::new(
                sub.velocity,
                AccelerationVector::zero(),
                sub.angular_velocity,
                AngularAccelerationVector::from_raw(Vector3::new(0.0, 0.0, 2.0)),
            ))
        })
        .unwrap();
        assert!((result.angular_velocity.raw().2 - 0.2).abs() < 1e-9);
        assert!((result.orientation.z() - 0.005).abs() < 1e-6);
        assert!(result.orientation.is_normalized(1e-12));
    }
}
