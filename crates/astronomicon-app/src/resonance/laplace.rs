use crate::error::AppResult;
use crate::resonance::orbit_info::load_system_hierarchy;
use crate::resonance::pairwise::{resolve_orbital_resonance, ResonanceReport};
use astronomicon_core::math::kepler::mean_longitude_at_epoch;
use astronomicon_core::math::resonance::{
    classify_libration, laplace_resonant_argument, ResonanceState,
};
use astronomicon_core::units::{Angle, Duration};
use astronomicon_db::SqlitePool;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaplaceChainReport {
    pub state: ResonanceState,
    pub current_critical_angle: Angle,
    pub inner_mmr: Option<ResonanceReport>,
    pub outer_mmr: Option<ResonanceReport>,
}

pub async fn resolve_laplace_chain(
    pool: &SqlitePool,
    star_system_id: Uuid,
    body_1_id: Uuid,
    body_2_id: Uuid,
    body_3_id: Uuid,
    universe_epoch: Duration,
    at_epoch: Duration,
    samples: usize,
) -> AppResult<LaplaceChainReport> {
    let hierarchy = load_system_hierarchy(pool, &star_system_id).await?;
    let info1 = hierarchy.get_body_orbit_info(&body_1_id)?;
    let info2 = hierarchy.get_body_orbit_info(&body_2_id)?;
    let info3 = hierarchy.get_body_orbit_info(&body_3_id)?;

    let mut sorted = [
        (body_1_id, info1),
        (body_2_id, info2),
        (body_3_id, info3),
    ];
    sorted.sort_by(|a, b| {
        b.1.mean_motion
            .partial_cmp(&a.1.mean_motion)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let (id1, inner) = sorted[0];
    let (id2, middle) = sorted[1];
    let (id3, outer) = sorted[2];

    let inner_mmr = resolve_orbital_resonance(
        pool,
        star_system_id,
        id1,
        id2,
        universe_epoch,
        at_epoch,
        samples,
    )
    .await?;

    let outer_mmr = resolve_orbital_resonance(
        pool,
        star_system_id,
        id2,
        id3,
        universe_epoch,
        at_epoch,
        samples,
    )
    .await?;

    let sample_count = samples.max(2);
    let delta_n = (middle.mean_motion.value() - outer.mean_motion.value()).abs();
    let synodic_period = if delta_n > 1e-15 {
        2.0 * PI / delta_n
    } else {
        middle.period.value()
    };
    let time_span = synodic_period * 4.0;
    let total_epoch = universe_epoch + at_epoch;

    let mut angles = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t_offset = Duration::new((i as f64 / (sample_count - 1) as f64) * time_span);
        let current_t = total_epoch + t_offset;

        let l1 = mean_longitude_at_epoch(&inner.elements, inner.mean_motion, current_t);
        let l2 = mean_longitude_at_epoch(&middle.elements, middle.mean_motion, current_t);
        let l3 = mean_longitude_at_epoch(&outer.elements, outer.mean_motion, current_t);

        let phi = laplace_resonant_argument(l1, l2, l3);
        angles.push(phi);
    }

    let state = classify_libration(&angles);
    let current_critical_angle = angles[0];

    Ok(LaplaceChainReport {
        state,
        current_critical_angle,
        inner_mmr,
        outer_mmr,
    })
}
