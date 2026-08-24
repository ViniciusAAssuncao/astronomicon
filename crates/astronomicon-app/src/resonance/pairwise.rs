use crate::error::AppResult;
use crate::resonance::orbit_info::load_system_hierarchy;
use astronomicon_core::math::kepler::mean_longitude_at_epoch;
use astronomicon_core::math::resonance::{
    classify_libration, mean_motion_resonance_search, resonance_order, resonant_argument,
    ResonanceState,
};
use astronomicon_core::units::{Angle, Duration};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResonanceReport {
    pub p: u32,
    pub q: u32,
    pub resonance_order: u32,
    pub normalized_deviation: f64,
    pub state: ResonanceState,
    pub current_critical_angle: Angle,
}

pub async fn resolve_orbital_resonance(
    pool: &SqlitePool,
    star_system_id: Uuid,
    body_a_id: Uuid,
    body_b_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    samples: usize,
) -> AppResult<Option<ResonanceReport>> {
    let hierarchy = load_system_hierarchy(pool, &star_system_id).await?;
    let info_a = hierarchy.get_body_orbit_info(&body_a_id)?;
    let info_b = hierarchy.get_body_orbit_info(&body_b_id)?;

    let (inner_info, outer_info) = if info_a.mean_motion.value() >= info_b.mean_motion.value() {
        (info_a, info_b)
    } else {
        (info_b, info_a)
    };

    let max_order = 32;
    let (p, q, dev) = match mean_motion_resonance_search(
        inner_info.mean_motion,
        outer_info.mean_motion,
        max_order,
    ) {
        Some(res) => res,
        None => return Ok(None),
    };

    let order = resonance_order(p, q);
    let sample_count = samples.max(2);

    let delta_n = (inner_info.mean_motion.value() - outer_info.mean_motion.value()).abs();
    let synodic_period = if delta_n > 1e-15 {
        2.0 * PI / delta_n
    } else {
        inner_info.period.value()
    };

    let time_span = synodic_period * (p as f64).max(1.0);
    let total_epoch = universe_epoch + at_epoch;

    let mut angles = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t_offset = Duration::new((i as f64 / (sample_count - 1) as f64) * time_span);
        let current_t = total_epoch + t_offset;

        let lambda1 = mean_longitude_at_epoch(
            &inner_info.elements,
            inner_info.mean_motion,
            current_t,
        );
        let lambda2 = mean_longitude_at_epoch(
            &outer_info.elements,
            outer_info.mean_motion,
            current_t,
        );
        let varpi = inner_info.elements.longitude_of_periapsis();

        let phi = resonant_argument(p, q, lambda1, lambda2, varpi);
        angles.push(phi);
    }

    let state = classify_libration(&angles);
    let current_critical_angle = angles[0];

    Ok(Some(ResonanceReport {
        p,
        q,
        resonance_order: order,
        normalized_deviation: dev,
        state,
        current_critical_angle,
    }))
}
