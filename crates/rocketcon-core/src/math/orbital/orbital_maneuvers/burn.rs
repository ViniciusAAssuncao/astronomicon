use astronomicon_core::units::{Duration, Force, Mass, Speed};

pub fn estimate_maneuver_burn_duration(
    delta_v: Speed,
    vehicle_initial_mass: Mass,
    effective_exhaust_velocity: Speed,
    total_thrust: Force,
) -> Duration {
    let dv = delta_v.value();
    let m0 = vehicle_initial_mass.value();
    let ve = effective_exhaust_velocity.value();
    let f = total_thrust.value();

    if dv <= 0.0
        || m0 <= 0.0
        || ve <= 0.0
        || f <= 0.0
        || !dv.is_finite()
        || !m0.is_finite()
        || !ve.is_finite()
        || !f.is_finite()
    {
        return Duration::new(0.0);
    }

    let m_dot = f / ve;
    let mass_ratio = (-dv / ve).exp();
    let mf = m0 * mass_ratio;
    let burn_time = (m0 - mf) / m_dot;

    Duration::new(burn_time.max(0.0))
}