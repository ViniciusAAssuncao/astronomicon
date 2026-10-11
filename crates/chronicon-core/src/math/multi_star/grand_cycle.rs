use crate::constants::constants::DEFAULT_MAX_SECONDARY_DENOMINATOR;
use crate::math::rational_approximation::{best_rational_approximation, lcm_u64};
use astronomicon_core::units::Duration;

pub fn find_polysolar_grand_cycle(
    primary_day: Duration,
    companion_days: &[Duration],
    max_denominator: Option<u32>,
) -> Option<Duration> {
    if companion_days.is_empty() {
        return None;
    }

    let t_pri = primary_day.value();
    if !t_pri.is_finite() || t_pri <= 0.0 {
        return None;
    }

    let max_den = max_denominator.unwrap_or(DEFAULT_MAX_SECONDARY_DENOMINATOR);
    let mut running_lcm: u64 = 1;

    for &comp_day in companion_days {
        let t_comp = comp_day.value();
        if !t_comp.is_finite() || t_comp <= 0.0 {
            continue;
        }

        let ratio = t_comp / t_pri;
        let approx = best_rational_approximation(ratio, Some(max_den))?;
        if approx.denominator > 0 {
            running_lcm = lcm_u64(running_lcm, approx.denominator as u64);
        }
    }

    if running_lcm == 0 || running_lcm > (u32::MAX as u64) {
        return None;
    }

    Some(Duration::new((running_lcm as f64) * t_pri))
}