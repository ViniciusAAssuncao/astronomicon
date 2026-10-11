use super::types::{EdelbaumTransferResult, SpiralEscapeResult};
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::constants::STANDARD_GRAVITY;
use astronomicon_core::units::{
    Angle, Duration, Force, GravitationalParameter, Length, Mass, MassRate, Speed,
};
use std::f64::consts::PI;

pub fn edelbaum_low_thrust_transfer(
    r_initial: Length,
    r_final: Length,
    inclination_change: Angle,
    initial_mass: Mass,
    thrust: Force,
    specific_impulse: Duration,
    mu: GravitationalParameter,
) -> RocketDomainResult<EdelbaumTransferResult> {
    let r0 = r_initial.value();
    let rf = r_final.value();
    let di = inclination_change.value().abs();
    let m0 = initial_mass.value();
    let f = thrust.value();
    let isp = specific_impulse.value();
    let mu_val = mu.value();

    if r0 <= 0.0
        || rf <= 0.0
        || m0 <= 0.0
        || f <= 0.0
        || isp <= 0.0
        || mu_val <= 0.0
        || !r0.is_finite()
        || !rf.is_finite()
        || !m0.is_finite()
        || !f.is_finite()
        || !isp.is_finite()
        || !mu_val.is_finite()
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
    let flight_time = if m_dot > 0.0 {
        prop_consumed / m_dot
    } else {
        0.0
    };

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
    mu: GravitationalParameter,
) -> RocketDomainResult<SpiralEscapeResult> {
    let r0 = r_initial.value();
    let rf = r_target.value();
    let m0 = initial_mass.value();
    let f = thrust.value();
    let isp = specific_impulse.value();
    let mu_val = mu.value();

    if r0 <= 0.0
        || rf <= r0
        || m0 <= 0.0
        || f <= 0.0
        || isp <= 0.0
        || mu_val <= 0.0
        || !r0.is_finite()
        || !rf.is_finite()
        || !m0.is_finite()
        || !f.is_finite()
        || !isp.is_finite()
        || !mu_val.is_finite()
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
    let flight_time = if m_dot > 0.0 {
        prop_consumed / m_dot
    } else {
        0.0
    };

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