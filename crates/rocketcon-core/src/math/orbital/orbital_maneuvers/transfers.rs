use super::types::{BiEllipticTransferResult, HohmannTransferResult};
use crate::error::{RocketDomainError, RocketDomainResult};
use astronomicon_core::units::{Duration, GravitationalParameter, Length, Speed};
use std::f64::consts::PI;

pub fn hohmann_transfer(
    r_initial: Length,
    r_target: Length,
    mu: GravitationalParameter,
) -> RocketDomainResult<HohmannTransferResult> {
    let r1 = r_initial.value();
    let r2 = r_target.value();
    let mu_val = mu.value();

    if r1 <= 0.0
        || r2 <= 0.0
        || mu_val <= 0.0
        || !r1.is_finite()
        || !r2.is_finite()
        || !mu_val.is_finite()
    {
        return Err(RocketDomainError::InvalidInvariant {
            field: "hohmann_transfer".to_string(),
            reason: "radii and gravitational parameter must be positive and finite".to_string(),
        });
    }

    let a_tx = (r1 + r2) * 0.5;
    let v1 = (mu_val / r1).sqrt();
    let v_tx1 = (mu_val * (2.0 / r1 - 1.0 / a_tx)).sqrt();
    let dv1 = v_tx1 - v1;

    let v2 = (mu_val / r2).sqrt();
    let v_tx2 = (mu_val * (2.0 / r2 - 1.0 / a_tx)).sqrt();
    let dv2 = v2 - v_tx2;

    let total_dv = dv1.abs() + dv2.abs();
    let tof = PI * (a_tx.powi(3) / mu_val).sqrt();

    Ok(HohmannTransferResult::new(
        Speed::new(dv1),
        Speed::new(dv2),
        Speed::new(total_dv),
        Duration::new(tof),
        Length::new(a_tx),
    ))
}

pub fn bi_elliptic_transfer(
    r_initial: Length,
    r_target: Length,
    r_intermediate: Length,
    mu: GravitationalParameter,
) -> RocketDomainResult<BiEllipticTransferResult> {
    let r1 = r_initial.value();
    let r2 = r_target.value();
    let rb = r_intermediate.value();
    let mu_val = mu.value();

    if r1 <= 0.0
        || r2 <= 0.0
        || rb <= 0.0
        || mu_val <= 0.0
        || !r1.is_finite()
        || !r2.is_finite()
        || !rb.is_finite()
        || !mu_val.is_finite()
    {
        return Err(RocketDomainError::InvalidInvariant {
            field: "bi_elliptic_transfer".to_string(),
            reason: "radii and gravitational parameter must be positive and finite".to_string(),
        });
    }

    if rb < r1 || rb < r2 {
        return Err(RocketDomainError::InvalidInvariant {
            field: "r_intermediate".to_string(),
            reason: "intermediate apoapsis radius must be greater than or equal to initial and target radii".to_string(),
        });
    }

    let a1 = (r1 + rb) * 0.5;
    let a2 = (r2 + rb) * 0.5;

    let v1 = (mu_val / r1).sqrt();
    let v_1_tx1 = (mu_val * (2.0 / r1 - 1.0 / a1)).sqrt();
    let dv1 = v_1_tx1 - v1;

    let v_b_tx1 = (mu_val * (2.0 / rb - 1.0 / a1)).sqrt();
    let v_b_tx2 = (mu_val * (2.0 / rb - 1.0 / a2)).sqrt();
    let dv2 = v_b_tx2 - v_b_tx1;

    let v_2_tx2 = (mu_val * (2.0 / r2 - 1.0 / a2)).sqrt();
    let v2 = (mu_val / r2).sqrt();
    let dv3 = v2 - v_2_tx2;

    let total_dv = dv1.abs() + dv2.abs() + dv3.abs();
    let tof = PI * (a1.powi(3) / mu_val).sqrt() + PI * (a2.powi(3) / mu_val).sqrt();

    Ok(BiEllipticTransferResult::new(
        Speed::new(dv1),
        Speed::new(dv2),
        Speed::new(dv3),
        Speed::new(total_dv),
        Duration::new(tof),
        Length::new(a1),
        Length::new(a2),
    ))
}