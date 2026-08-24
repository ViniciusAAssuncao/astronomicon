use crate::units::Length;
use crate::units::constants::{LOW_CLOUD_TROPOPAUSE_FRACTION, MID_CLOUD_TROPOPAUSE_FRACTION, HIGH_CLOUD_TROPOPAUSE_FRACTION};

pub fn cloud_band_altitudes(tropopause_altitude: Length) -> (Length, Length, Length, Length) {
    let z_t = tropopause_altitude.value().max(0.0);
    (
        Length::new(0.0),
        Length::new(LOW_CLOUD_TROPOPAUSE_FRACTION * z_t),
        Length::new(MID_CLOUD_TROPOPAUSE_FRACTION * z_t),
        Length::new(HIGH_CLOUD_TROPOPAUSE_FRACTION * z_t),
    )
}

pub fn layer_critical_relative_humidity(normalized_pressure: f64) -> f64 {
    if !normalized_pressure.is_finite() || normalized_pressure <= 0.0 {
        return 0.70;
    }
    let sigma = normalized_pressure.clamp(0.0, 1.0);
    let rh_crit_top = 0.70;
    let rh_crit_surf = 0.90;
    rh_crit_top + (rh_crit_surf - rh_crit_top) * sigma * sigma
}

pub fn sundqvist_cloud_fraction(
    relative_humidity: f64,
    critical_relative_humidity: f64,
) -> f64 {
    let rh = relative_humidity.clamp(0.0, 1.0);
    let rh_crit = critical_relative_humidity.clamp(0.0, 1.0);

    if !rh.is_finite() || !rh_crit.is_finite() || rh <= rh_crit {
        return 0.0;
    }

    if rh >= 1.0 {
        return 1.0;
    }

    if rh_crit >= 1.0 {
        return 0.0;
    }

    let ratio = ((1.0 - rh) / (1.0 - rh_crit)).clamp(0.0, 1.0);
    let c = 1.0 - ratio.sqrt();
    c.clamp(0.0, 1.0)
}

pub fn combine_layer_cloud_fractions_max_random_overlap(
    low: f64,
    mid: f64,
    high: f64,
) -> f64 {
    let c1 = if low.is_finite() { low.clamp(0.0, 1.0) } else { 0.0 };
    let c2 = if mid.is_finite() { mid.clamp(0.0, 1.0) } else { 0.0 };
    let c3 = if high.is_finite() { high.clamp(0.0, 1.0) } else { 0.0 };

    if c1 >= 1.0 || c2 >= 1.0 || c3 >= 1.0 {
        return 1.0;
    }

    let t_clear = (1.0 - c1.max(c2)) * (1.0 - c2.max(c3)) / (1.0 - c2);
    (1.0 - t_clear).clamp(0.0, 1.0)
}