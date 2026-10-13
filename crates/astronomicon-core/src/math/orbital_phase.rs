use crate::error::{DomainError, DomainResult};
use crate::units::{Angle, AngularVelocity, Duration};
use std::f64::consts::TAU;

const TAU_RESIDUAL: f64 = 2.449_293_598_294_706_4e-16;
const MAX_EXACT_REVOLUTIONS: f64 = 281_474_976_710_656.0;

pub fn compensated_orbital_phase(
    phase_at_origin: Angle,
    rate: AngularVelocity,
    absolute_epoch: Duration,
    local_elapsed: Duration,
) -> DomainResult<Angle> {
    let theta = phase_at_origin.value();
    let angular_rate = rate.value();
    let base = absolute_epoch.value();
    let local = local_elapsed.value();

    if !theta.is_finite() || !angular_rate.is_finite() || !base.is_finite() || !local.is_finite() {
        return Err(DomainError::InvalidInvariant {
            field: "orbital_phase".to_owned(),
            reason: "angle, angular rate and both time components must be finite".to_owned(),
        });
    }

    let product = angular_rate * base;
    let product_tail = angular_rate.mul_add(base, -product);
    let local_product = angular_rate * local;
    let local_tail = angular_rate.mul_add(local, -local_product);
    let sum = product + local_product;
    let sum_tail = (product - sum) + local_product + product_tail + local_tail;
    let turns = (sum / TAU).round();

    if !turns.is_finite() || turns.abs() >= MAX_EXACT_REVOLUTIONS {
        return Err(DomainError::InvalidInvariant {
            field: "orbital_phase".to_owned(),
            reason: "orbital revolution count exceeds compensated reduction range".to_owned(),
        });
    }

    let tau_product = turns * TAU;
    let tau_tail = turns.mul_add(TAU, -tau_product);
    let reduced = (sum - tau_product) + sum_tail - tau_tail - turns * TAU_RESIDUAL;
    Ok(Angle::new((theta + reduced).rem_euclid(TAU)))
}
