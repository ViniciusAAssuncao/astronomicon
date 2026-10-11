use crate::math::synodic::synodic_beat_period;
use astronomicon_core::units::Duration;

pub fn solar_day_length(
    sidereal_rotation_period: Duration,
    orbital_period: Duration,
    is_retrograde: bool,
) -> Option<Duration> {
    synodic_beat_period(sidereal_rotation_period, orbital_period, !is_retrograde)
}
