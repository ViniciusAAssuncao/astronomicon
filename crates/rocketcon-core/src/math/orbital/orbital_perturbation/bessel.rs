use crate::constants::{BESSEL_MAX_TERMS, BESSEL_SERIES_TOLERANCE};
use std::f64::consts::PI;

pub fn bessel_i0_scaled(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 3.75 {
        let mut term = 1.0;
        let mut sum = 1.0;
        let x_half_sq = 0.25 * ax * ax;
        for k in 1..=BESSEL_MAX_TERMS {
            term *= x_half_sq / ((k * k) as f64);
            sum += term;
            if term < sum * BESSEL_SERIES_TOLERANCE {
                break;
            }
        }
        (-ax).exp() * sum
    } else {
        let inv_x = 1.0 / ax;
        let p = 1.0
            + 0.125 * inv_x
            + (9.0 / 128.0) * inv_x * inv_x
            + (225.0 / 3072.0) * inv_x * inv_x * inv_x
            + (11025.0 / 98304.0) * inv_x * inv_x * inv_x * inv_x;
        p / (2.0 * PI * ax).sqrt()
    }
}

pub fn bessel_i1_scaled(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 1e-15 {
        return 0.0;
    }
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    if ax < 3.75 {
        let mut term = 0.5 * ax;
        let mut sum = term;
        let x_half_sq = 0.25 * ax * ax;
        for k in 1..=BESSEL_MAX_TERMS {
            term *= x_half_sq / ((k * (k + 1)) as f64);
            sum += term;
            if term < sum * BESSEL_SERIES_TOLERANCE {
                break;
            }
        }
        sign * (-ax).exp() * sum
    } else {
        let inv_x = 1.0 / ax;
        let p = 1.0
            - 0.375 * inv_x
            - (15.0 / 128.0) * inv_x * inv_x
            - (315.0 / 3072.0) * inv_x * inv_x * inv_x
            - (1575.0 / 98304.0) * inv_x * inv_x * inv_x * inv_x;
        sign * p / (2.0 * PI * ax).sqrt()
    }
}

pub fn bessel_i2_scaled(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 1e-15 {
        return 0.0;
    }
    if ax < 0.5 {
        let mut term = 0.125 * ax * ax;
        let mut sum = term;
        let x_half_sq = 0.25 * ax * ax;
        for k in 1..=BESSEL_MAX_TERMS {
            term *= x_half_sq / ((k * (k + 2)) as f64);
            sum += term;
            if term < sum * BESSEL_SERIES_TOLERANCE {
                break;
            }
        }
        (-ax).exp() * sum
    } else {
        bessel_i0_scaled(ax) - (2.0 / ax) * bessel_i1_scaled(ax)
    }
}

pub fn bessel_i3_scaled(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 1e-15 {
        return 0.0;
    }
    let sign = if x >= 0.0 { 1.0 } else { -1.0 };
    if ax < 0.5 {
        let mut term = (1.0 / 48.0) * ax * ax * ax;
        let mut sum = term;
        let x_half_sq = 0.25 * ax * ax;
        for k in 1..=BESSEL_MAX_TERMS {
            term *= x_half_sq / ((k * (k + 3)) as f64);
            sum += term;
            if term < sum * BESSEL_SERIES_TOLERANCE {
                break;
            }
        }
        sign * (-ax).exp() * sum
    } else {
        sign * (bessel_i1_scaled(ax) - (4.0 / ax) * bessel_i2_scaled(ax))
    }
}
