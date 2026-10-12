use crate::error::{RocketDomainError, RocketDomainResult};
use crate::math::orbital::types::OsculatingElements;
use astronomicon_core::units::{Angle, GravitationalParameter, Length, Speed};

pub fn apsidal_rotation_delta_v(
    elements: &OsculatingElements,
    target_argument_of_periapsis: Angle,
    mu: GravitationalParameter,
) -> RocketDomainResult<Speed> {
    let e = elements.eccentricity;
    let a = elements.semi_major_axis.value();
    let mu_val = mu.value();
    let delta_omega =
        (target_argument_of_periapsis.value() - elements.argument_of_periapsis.value()).abs();

    if e <= 0.0
        || e >= 1.0
        || a <= 0.0
        || mu_val <= 0.0
        || !e.is_finite()
        || !a.is_finite()
        || !mu_val.is_finite()
    {
        return Err(RocketDomainError::InvalidInvariant {
            field: "apsidal_rotation".to_string(),
            reason: "orbit must be bound and eccentric with positive parameters".to_string(),
        });
    }

    let p = a * (1.0 - e * e);
    let dv = 2.0 * (mu_val / p).sqrt() * e * (0.5 * delta_omega).sin().abs();

    Ok(Speed::new(dv))
}

pub fn hyperbolic_excess_velocity_to_periapsis_speed(
    v_infinity: Speed,
    periapsis_radius: Length,
    mu: GravitationalParameter,
) -> Speed {
    let v_inf = v_infinity.value();
    let rp = periapsis_radius.value();
    let mu_val = mu.value();

    if rp <= 0.0
        || mu_val <= 0.0
        || !rp.is_finite()
        || !mu_val.is_finite()
        || !v_inf.is_finite()
        || v_inf < 0.0
    {
        return Speed::new(0.0);
    }

    let vp = (v_inf * v_inf + 2.0 * mu_val / rp).sqrt();
    Speed::new(vp)
}

pub fn orbital_insertion_delta_v(
    v_infinity: Speed,
    target_periapsis: Length,
    target_apoapsis: Option<Length>,
    mu: GravitationalParameter,
) -> RocketDomainResult<Speed> {
    let v_inf = v_infinity.value();
    let rp = target_periapsis.value();
    let mu_val = mu.value();

    if rp <= 0.0
        || mu_val <= 0.0
        || !rp.is_finite()
        || !mu_val.is_finite()
        || !v_inf.is_finite()
        || v_inf < 0.0
    {
        return Err(RocketDomainError::InvalidInvariant {
            field: "orbital_insertion".to_string(),
            reason: "parameters must be positive and finite".to_string(),
        });
    }

    let ra = target_apoapsis.map_or(rp, |a| a.value());
    if ra < rp || !ra.is_finite() {
        return Err(RocketDomainError::InvalidInvariant {
            field: "target_apoapsis".to_string(),
            reason: "target apoapsis must be greater than or equal to periapsis".to_string(),
        });
    }

    let v_hyp_p = (v_inf * v_inf + 2.0 * mu_val / rp).sqrt();
    let a_target = 0.5 * (rp + ra);
    let v_target_p = (mu_val * (2.0 / rp - 1.0 / a_target)).sqrt();
    let dv = (v_hyp_p - v_target_p).max(0.0);

    Ok(Speed::new(dv))
}