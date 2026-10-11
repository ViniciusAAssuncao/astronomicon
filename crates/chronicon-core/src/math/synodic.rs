use crate::constants::constants::MINIMUM_SYNODIC_PERIOD_SECONDS;
use astronomicon_core::units::{AngularVelocity, Duration};
use std::f64::consts::TAU;

pub fn synodic_beat_period(
    period_a: Duration,
    period_b: Duration,
    same_direction: bool,
) -> Option<Duration> {
    let t_a = period_a.value();
    let t_b = period_b.value();

    if !t_a.is_finite() || !t_b.is_finite() || t_a <= 0.0 || t_b <= 0.0 {
        return None;
    }

    let f_a = 1.0 / t_a;
    let f_b = 1.0 / t_b;

    let f_beat = if same_direction {
        (f_a - f_b).abs()
    } else {
        f_a + f_b
    };

    if !f_beat.is_finite() || f_beat <= 0.0 {
        return None;
    }

    let t_beat = 1.0 / f_beat;
    if !t_beat.is_finite() || t_beat < MINIMUM_SYNODIC_PERIOD_SECONDS {
        return None;
    }

    Some(Duration::new(t_beat))
}

pub fn synodic_period_from_mean_motions(
    mean_motion_a: AngularVelocity,
    mean_motion_b: AngularVelocity,
    same_direction: bool,
) -> Option<Duration> {
    let n_a = mean_motion_a.value().abs();
    let n_b = mean_motion_b.value().abs();

    if !n_a.is_finite() || !n_b.is_finite() || n_a <= 0.0 || n_b <= 0.0 {
        return None;
    }

    let delta_n = if same_direction {
        (n_a - n_b).abs()
    } else {
        n_a + n_b
    };

    if !delta_n.is_finite() || delta_n <= 0.0 {
        return None;
    }

    let t_syn = TAU / delta_n;
    if !t_syn.is_finite() || t_syn < MINIMUM_SYNODIC_PERIOD_SECONDS {
        return None;
    }

    Some(Duration::new(t_syn))
}
