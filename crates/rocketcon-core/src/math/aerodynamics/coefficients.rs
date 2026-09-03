use super::types::{mach_regime, MachRegime};
use astronomicon_core::units::Angle;
use std::f64::consts::PI;

const HYPERSONIC_BETA_REF: f64 = 4.898979485566356;

pub fn zero_lift_drag_coefficient(mach: f64) -> f64 {
    if !mach.is_finite() || mach <= 0.0 {
        return 0.18;
    }

    match mach_regime(mach) {
        MachRegime::Subsonic => {
            let m_norm = mach / 0.8;
            0.18 + 0.04 * m_norm * m_norm
        }
        MachRegime::Transonic => {
            let t = (mach - 0.8) / 0.4;
            let wave_peak = (PI * t).sin().powi(2);
            0.22 + 0.38 * wave_peak + 0.15 * t
        }
        MachRegime::Supersonic => {
            let beta = (mach * mach - 1.0).max(0.1).sqrt();
            0.18 + 0.35 / beta
        }
        MachRegime::Hypersonic => {
            let base_hypersonic = 0.18 + 0.35 / HYPERSONIC_BETA_REF;
            (base_hypersonic - 0.02 * ((mach - 5.0) / 10.0)).max(0.18)
        }
    }
}

pub fn normal_force_slope(mach: f64) -> f64 {
    if !mach.is_finite() || mach <= 0.0 {
        return 2.0;
    }

    match mach_regime(mach) {
        MachRegime::Subsonic => 2.0,
        MachRegime::Transonic => {
            let t = (mach - 0.8) / 0.4;
            2.0 + 0.6 * (PI * t).sin()
        }
        MachRegime::Supersonic => {
            let beta = (mach * mach - 1.0).max(0.2).sqrt();
            2.0 / beta
        }
        MachRegime::Hypersonic => {
            (2.0 / HYPERSONIC_BETA_REF).max(0.4)
        }
    }
}

pub fn normal_force_coefficient(mach: f64, total_angle_of_attack: Angle) -> f64 {
    let alpha = total_angle_of_attack.value();
    if !alpha.is_finite() {
        return 0.0;
    }

    let sin_a = alpha.sin();
    let cos_a = alpha.cos();
    let abs_sin_a = sin_a.abs();

    match mach_regime(mach) {
        MachRegime::Hypersonic => {
            2.0 * abs_sin_a * abs_sin_a * cos_a.max(0.0) + 1.2 * abs_sin_a * sin_a
        }
        _ => {
            let cn_alpha = normal_force_slope(mach);
            let linear_term = cn_alpha * sin_a * cos_a;
            let crossflow_term = 1.2 * abs_sin_a * sin_a;
            linear_term + crossflow_term
        }
    }
}

pub fn axial_drag_coefficient(mach: f64, total_angle_of_attack: Angle) -> f64 {
    let cd0 = zero_lift_drag_coefficient(mach);
    let alpha = total_angle_of_attack.value();
    if !alpha.is_finite() {
        return cd0;
    }

    let sin_a = alpha.sin();
    let cos_a = alpha.cos();
    let abs_sin_a = sin_a.abs();
    let cn = normal_force_coefficient(mach, total_angle_of_attack);

    let induced_drag = (cn * sin_a).abs();
    let crossflow_drag = 1.2 * abs_sin_a * abs_sin_a * abs_sin_a;

    (cd0 * cos_a.abs() + induced_drag + crossflow_drag).max(0.01)
}

pub fn generic_slender_body_drag_coefficient_estimate(mach: f64) -> f64 {
    axial_drag_coefficient(mach, Angle::new(0.0))
}

pub fn drag_coefficient_estimate(mach: f64) -> f64 {
    generic_slender_body_drag_coefficient_estimate(mach)
}