use crate::constants::constants::MINIMUM_SOLAR_DAY_SECONDS;
use astronomicon_core::units::Duration;

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
    if !t_beat.is_finite() || t_beat < MINIMUM_SOLAR_DAY_SECONDS {
        return None;
    }

    Some(Duration::new(t_beat))
}

pub fn solar_day_length(
    sidereal_rotation_period: Duration,
    orbital_period: Duration,
    is_retrograde: bool,
) -> Option<Duration> {
    synodic_beat_period(sidereal_rotation_period, orbital_period, !is_retrograde)
}