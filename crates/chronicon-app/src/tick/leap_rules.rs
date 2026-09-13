use chronicon_core::math::{IntercalationAdjustmentDirection, RefinedIntercalationRule};

pub fn is_container_leap(container_index: i64, rule: &RefinedIntercalationRule) -> bool {
    let p_rule = &rule.primary_rule;
    if p_rule.cycle_containers == 0 || p_rule.leap_units == 0 {
        return false;
    }

    let q1 = p_rule.cycle_containers as i64;
    let p1 = p_rule.leap_units as i64;
    let rem1 = container_index.rem_euclid(q1);
    let mut leap = (rem1 * p1) % q1 < p1;

    if let Some(sec) = &rule.secondary_correction {
        if sec.cycle_containers > 0 {
            let q2 = sec.cycle_containers as i64;
            if year_index_rem(container_index, q2) == 0 {
                match sec.direction {
                    IntercalationAdjustmentDirection::SubtractLeapUnits => leap = false,
                    IntercalationAdjustmentDirection::AddLeapUnits => leap = true,
                }
            }
        }
    }

    leap
}

fn year_index_rem(container_index: i64, q: i64) -> i64 {
    container_index.rem_euclid(q)
}

pub fn units_in_container(container_index: i64, rule: &RefinedIntercalationRule) -> u32 {
    rule.base_units_per_container + if is_container_leap(container_index, rule) { 1 } else { 0 }
}

pub fn cumulative_units_to_container(
    container_index: i64,
    rule: &RefinedIntercalationRule,
) -> i64 {
    let q_tot = (rule.total_cycle_containers as i64).max(1);
    let units_per_cycle =
        (q_tot * (rule.base_units_per_container as i64)) + (rule.total_leap_units as i64);

    let cycle_count = container_index.div_euclid(q_tot);
    let container_in_cycle = container_index.rem_euclid(q_tot);

    let mut units = cycle_count * units_per_cycle;
    let base_container = cycle_count * q_tot;
    for c in 0..container_in_cycle {
        units += units_in_container(base_container + c, rule) as i64;
    }

    units
}

pub fn resolve_container_and_unit(
    total_units: f64,
    rule: &RefinedIntercalationRule,
) -> (i64, f64, i64, bool, bool) {
    let mean_units = rule.mean_units_per_container.max(1.0);
    let mut c = (total_units / mean_units).floor() as i64;

    let mut start_unit = cumulative_units_to_container(c, rule) as f64;
    let mut container_len = units_in_container(c, rule) as f64;

    while total_units < start_unit {
        c -= 1;
        start_unit = cumulative_units_to_container(c, rule) as f64;
        container_len = units_in_container(c, rule) as f64;
    }

    while total_units >= start_unit + container_len {
        c += 1;
        start_unit = cumulative_units_to_container(c, rule) as f64;
        container_len = units_in_container(c, rule) as f64;
    }

    let unit_in_container = total_units - start_unit;
    let unit_index = unit_in_container.floor() as i64;
    let is_leap = is_container_leap(c, rule);
    let is_boundary = is_leap && (unit_index >= rule.base_units_per_container as i64);

    (c, unit_in_container, unit_index, is_leap, is_boundary)
}
