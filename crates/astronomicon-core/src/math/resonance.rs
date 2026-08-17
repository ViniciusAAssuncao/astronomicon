use crate::units::{Angle, AngularVelocity};
use std::f64::consts::{PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ResonanceState {
    Librating,
    Circulating,
}

pub fn resonance_order(p: u32, q: u32) -> u32 {
    p.abs_diff(q)
}

pub fn mean_motion_resonance_search(
    n1: AngularVelocity,
    n2: AngularVelocity,
    max_order: u32,
) -> Option<(u32, u32, f64)> {
    let v1 = n1.value();
    let v2 = n2.value();
    if v1 <= 0.0 || v2 <= 0.0 || !v1.is_finite() || !v2.is_finite() || max_order < 2 {
        return None;
    }

    let (target, inverted) = if v1 >= v2 {
        (v1 / v2, false)
    } else {
        (v2 / v1, true)
    };

    let mut x = target;
    let mut h_prev2: i64 = 0;
    let mut h_prev1: i64 = 1;
    let mut k_prev2: i64 = 1;
    let mut k_prev1: i64 = 0;

    let mut best: Option<(u32, u32, f64)> = None;

    for _ in 0..100 {
        let a = x.floor();
        let a_int = a as i64;

        let h = a_int.checked_mul(h_prev1)?.checked_add(h_prev2)?;
        let k = a_int.checked_mul(k_prev1)?.checked_add(k_prev2)?;

        if h <= 0 || k <= 0 {
            break;
        }

        let p = h as u32;
        let q = k as u32;

        if (p as u64) + (q as u64) > max_order as u64 {
            break;
        }

        let approx = (p as f64) / (q as f64);
        let dev = (target - approx).abs() / approx;

        let candidate = if inverted { (q, p, dev) } else { (p, q, dev) };

        match best {
            None => {
                best = Some(candidate);
            }
            Some((_, _, best_dev)) => {
                if dev < best_dev {
                    best = Some(candidate);
                }
            }
        }

        let frac = x - a;
        if frac.abs() < 1e-12 {
            break;
        }
        x = 1.0 / frac;

        h_prev2 = h_prev1;
        h_prev1 = h;
        k_prev2 = k_prev1;
        k_prev1 = k;
    }

    best
}

pub fn resonant_argument_first_order(
    p: u32,
    lambda_inner: Angle,
    lambda_outer: Angle,
    varpi: Angle,
) -> Angle {
    let val = ((p + 1) as f64) * lambda_outer.value() - (p as f64) * lambda_inner.value() - varpi.value();
    Angle::new(val.rem_euclid(TAU))
}

pub fn resonant_argument(
    p: u32,
    q: u32,
    lambda_inner: Angle,
    lambda_outer: Angle,
    varpi: Angle,
) -> Angle {
    let p_val = p.max(q) as f64;
    let q_val = p.min(q) as f64;
    let order = (p_val - q_val) as f64;
    let val = p_val * lambda_outer.value() - q_val * lambda_inner.value() - order * varpi.value();
    Angle::new(val.rem_euclid(TAU))
}

pub fn laplace_resonant_argument(
    lambda1: Angle,
    lambda2: Angle,
    lambda3: Angle,
) -> Angle {
    let val = lambda1.value() - 3.0 * lambda2.value() + 2.0 * lambda3.value();
    Angle::new(val.rem_euclid(TAU))
}

pub fn unwrap_angles(angles: &[Angle]) -> Vec<f64> {
    if angles.is_empty() {
        return Vec::new();
    }
    let mut unwrapped = Vec::with_capacity(angles.len());
    let mut prev = angles[0].value();
    unwrapped.push(prev);
    for a in &angles[1..] {
        let current = a.value();
        let mut diff = (current - prev).rem_euclid(TAU);
        if diff > PI {
            diff -= TAU;
        }
        let next = prev + diff;
        unwrapped.push(next);
        prev = next;
    }
    unwrapped
}

pub fn classify_libration(angles: &[Angle]) -> ResonanceState {
    if angles.len() < 2 {
        return ResonanceState::Circulating;
    }
    let unwrapped = unwrap_angles(angles);
    let mut min_val = unwrapped[0];
    let mut max_val = unwrapped[0];
    for &val in &unwrapped[1..] {
        if val < min_val {
            min_val = val;
        }
        if val > max_val {
            max_val = val;
        }
    }
    if (max_val - min_val) < TAU {
        ResonanceState::Librating
    } else {
        ResonanceState::Circulating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean_motion_resonance_search_exact() {
        let n1 = AngularVelocity::new(2.0);
        let n2 = AngularVelocity::new(1.0);
        let (p, q, dev) = mean_motion_resonance_search(n1, n2, 10).unwrap();
        assert_eq!(p, 2);
        assert_eq!(q, 1);
        assert!(dev < 1e-12);
    }

    #[test]
    fn test_mean_motion_resonance_search_3_2() {
        let n1 = AngularVelocity::new(3.0003);
        let n2 = AngularVelocity::new(2.0);
        let (p, q, dev) = mean_motion_resonance_search(n1, n2, 10).unwrap();
        assert_eq!(p, 3);
        assert_eq!(q, 2);
        assert!(dev < 1e-3);
    }

    #[test]
    fn test_resonance_order() {
        assert_eq!(resonance_order(2, 1), 1);
        assert_eq!(resonance_order(3, 2), 1);
        assert_eq!(resonance_order(5, 2), 3);
        assert_eq!(resonance_order(7, 3), 4);
    }

    #[test]
    fn test_resonant_argument() {
        let l1 = Angle::new(1.0);
        let l2 = Angle::new(2.0);
        let varpi = Angle::new(0.5);
        let arg = resonant_argument_first_order(1, l1, l2, varpi);
        let expected = (2.0 * 2.0 - 1.0 * 1.0 - 0.5_f64).rem_euclid(TAU);
        assert!((arg.value() - expected).abs() < 1e-12);
    }

    #[test]
    fn test_laplace_argument() {
        let l1 = Angle::new(PI);
        let l2 = Angle::new(0.0);
        let l3 = Angle::new(0.0);
        let phi = laplace_resonant_argument(l1, l2, l3);
        assert!((phi.value() - PI).abs() < 1e-12);
    }

    #[test]
    fn test_classify_libration() {
        let librating = vec![
            Angle::new(0.1),
            Angle::new(0.2),
            Angle::new(0.15),
            Angle::new(0.05),
        ];
        assert_eq!(classify_libration(&librating), ResonanceState::Librating);

        let circulating = vec![
            Angle::new(0.0),
            Angle::new(2.0),
            Angle::new(4.0),
            Angle::new(6.0),
            Angle::new(1.5),
            Angle::new(3.5),
        ];
        assert_eq!(classify_libration(&circulating), ResonanceState::Circulating);
    }
}