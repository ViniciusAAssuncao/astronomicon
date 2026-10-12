use crate::constants::constants::{
    DEFAULT_CONTINUED_FRACTION_EPSILON, DEFAULT_MAX_CONTINUED_FRACTION_DEPTH,
    DEFAULT_MAX_INTERCALATION_DENOMINATOR,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RationalApproximation {
    pub numerator: u32,
    pub denominator: u32,
}

impl RationalApproximation {
    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    pub fn value(&self) -> f64 {
        if self.denominator == 0 {
            0.0
        } else {
            self.numerator as f64 / self.denominator as f64
        }
    }

    pub fn error_from(&self, target: f64) -> f64 {
        self.value() - target
    }

    pub fn absolute_error_from(&self, target: f64) -> f64 {
        (self.value() - target).abs()
    }
}

pub fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

pub fn lcm_u64(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        (a / gcd_u64(a, b)) * b
    }
}

pub fn continued_fraction_convergents(
    target: f64,
    max_denominator: Option<u32>,
    max_depth: Option<u32>,
    epsilon: Option<f64>,
) -> Vec<RationalApproximation> {
    if !target.is_finite() || target < 0.0 {
        return Vec::new();
    }

    let max_den = max_denominator
        .unwrap_or(DEFAULT_MAX_INTERCALATION_DENOMINATOR)
        .max(1) as u64;
    let max_d = max_depth.unwrap_or(DEFAULT_MAX_CONTINUED_FRACTION_DEPTH);
    let eps = epsilon.unwrap_or(DEFAULT_CONTINUED_FRACTION_EPSILON);

    let mut convergents = Vec::new();

    let mut p_prev2: u64 = 0;
    let mut q_prev2: u64 = 1;
    let mut p_prev1: u64 = 1;
    let mut q_prev1: u64 = 0;

    let mut x = target;

    for _ in 0..max_d {
        let a = x.floor();
        if !a.is_finite() || a < 0.0 {
            break;
        }

        let a_u64 = a as u64;

        let p_curr = match a_u64.checked_mul(p_prev1).and_then(|v| v.checked_add(p_prev2)) {
            Some(v) => v,
            None => break,
        };

        let q_curr = match a_u64.checked_mul(q_prev1).and_then(|v| v.checked_add(q_prev2)) {
            Some(v) => v,
            None => break,
        };

        if q_curr > max_den || p_curr > u32::MAX as u64 || q_curr > u32::MAX as u64 {
            if q_prev1 > 0 && q_prev1 <= max_den {
                let rem_den = max_den.saturating_sub(q_prev2);
                let m = rem_den / q_prev1;
                if m > 0 && m < a_u64 && (m * 2 >= a_u64) {
                    let p_semi = m * p_prev1 + p_prev2;
                    let q_semi = m * q_prev1 + q_prev2;
                    if p_semi <= u32::MAX as u64 && q_semi <= u32::MAX as u64 && q_semi > 0 {
                        let approx = RationalApproximation::new(p_semi as u32, q_semi as u32);
                        if !convergents.contains(&approx) {
                            convergents.push(approx);
                        }
                    }
                }
            }
            break;
        }

        if q_curr > 0 {
            convergents.push(RationalApproximation::new(p_curr as u32, q_curr as u32));
        }

        p_prev2 = p_prev1;
        q_prev2 = q_prev1;
        p_prev1 = p_curr;
        q_prev1 = q_curr;

        let frac = x - a;
        if frac.abs() <= eps {
            break;
        }

        x = 1.0 / frac;
    }

    convergents
}

pub fn best_rational_approximation(
    target: f64,
    max_denominator: Option<u32>,
) -> Option<RationalApproximation> {
    let list = continued_fraction_convergents(target, max_denominator, None, None);
    let mut best: Option<(RationalApproximation, f64)> = None;

    for approx in list {
        let err = approx.absolute_error_from(target);
        match best {
            None => best = Some((approx, err)),
            Some((_, best_err)) => {
                if err < best_err {
                    best = Some((approx, err));
                }
            }
        }
    }

    best.map(|(a, _)| a)
}