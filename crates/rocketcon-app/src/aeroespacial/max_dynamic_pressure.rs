use astronomicon_core::units::{Duration, Pressure};
use rocketcon_core::domain::VehiclePhysicalState;

pub(crate) fn resolve_max_q(
    physical_state: &VehiclePhysicalState,
    sampled_q: Option<Pressure>,
    sampled_epoch: Duration,
) -> (Option<Pressure>, Option<Duration>) {
    match (physical_state.max_dynamic_pressure(), sampled_q) {
        (Some(prev_q), Some(current_q)) => {
            if current_q.value() > prev_q.value() {
                (Some(current_q), Some(sampled_epoch))
            } else {
                (Some(prev_q), physical_state.max_dynamic_pressure_epoch())
            }
        }
        (None, Some(current_q)) => (Some(current_q), Some(sampled_epoch)),
        (Some(prev_q), None) => (Some(prev_q), physical_state.max_dynamic_pressure_epoch()),
        (None, None) => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::{
        AngularVelocityVector, Position, Quaternion, Vector3, VelocityVector,
    };
    use uuid::Uuid;

    fn state(previous: Option<Pressure>, epoch: Option<Duration>) -> VehiclePhysicalState {
        VehiclePhysicalState::new_with_max_q(
            Uuid::from_u128(1),
            Position::from_raw(Vector3::zero()),
            VelocityVector::from_raw(Vector3::zero()),
            Quaternion::identity(),
            AngularVelocityVector::from_raw(Vector3::zero()),
            Uuid::from_u128(2),
            Duration::new(1.0),
            Duration::new(1.0),
            previous,
            epoch,
        )
        .unwrap()
    }

    #[test]
    fn first_aerodynamic_sample_sets_pressure_and_epoch() {
        let epoch = Duration::new(5.0);
        let sample = Pressure::new(120.0);
        assert_eq!(
            resolve_max_q(&state(None, None), Some(sample), epoch),
            (Some(sample), Some(epoch))
        );
        assert_eq!(resolve_max_q(&state(None, None), None, epoch), (None, None));
    }

    #[test]
    fn increased_pressure_replaces_peak_at_sample_epoch() {
        let past = Duration::new(3.0);
        let now = Duration::new(6.0);
        assert_eq!(
            resolve_max_q(
                &state(Some(Pressure::new(120.0)), Some(past)),
                Some(Pressure::new(130.0)),
                now
            ),
            (Some(Pressure::new(130.0)), Some(now))
        );
    }

    #[test]
    fn equal_lower_or_missing_sample_preserves_previous_peak_and_epoch() {
        let past = Duration::new(3.0);
        let now = Duration::new(6.0);
        let initial = state(Some(Pressure::new(120.0)), Some(past));
        for sample in [Some(Pressure::new(120.0)), Some(Pressure::new(110.0)), None] {
            assert_eq!(
                resolve_max_q(&initial, sample, now),
                (Some(Pressure::new(120.0)), Some(past))
            );
        }
    }
}
