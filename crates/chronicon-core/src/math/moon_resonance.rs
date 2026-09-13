use crate::constants::constants::{
    DEFAULT_MAX_RESONANCE_DEVIATION, DEFAULT_MAX_RESONANCE_ORDER,
};
use astronomicon_core::math::resonance::{
    mean_motion_resonance_search, resonance_order,
};
use astronomicon_core::units::AngularVelocity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoonPairResonance {
    pub inner_moon_id: Uuid,
    pub outer_moon_id: Uuid,
    pub p: u32,
    pub q: u32,
    pub deviation: f64,
    pub resonance_order: u32,
}

impl MoonPairResonance {
    pub fn new(
        inner_moon_id: Uuid,
        outer_moon_id: Uuid,
        p: u32,
        q: u32,
        deviation: f64,
    ) -> Self {
        let order = resonance_order(p, q);
        Self {
            inner_moon_id,
            outer_moon_id,
            p,
            q,
            deviation,
            resonance_order: order,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LaplaceTrioResonance {
    pub inner_moon_id: Uuid,
    pub middle_moon_id: Uuid,
    pub outer_moon_id: Uuid,
    pub ratio_inner: u32,
    pub ratio_middle: u32,
    pub ratio_outer: u32,
    pub deviation: f64,
}

impl LaplaceTrioResonance {
    pub fn new(
        inner_moon_id: Uuid,
        middle_moon_id: Uuid,
        outer_moon_id: Uuid,
        ratio_inner: u32,
        ratio_middle: u32,
        ratio_outer: u32,
        deviation: f64,
    ) -> Self {
        Self {
            inner_moon_id,
            middle_moon_id,
            outer_moon_id,
            ratio_inner,
            ratio_middle,
            ratio_outer,
            deviation,
        }
    }
}

fn gcd_u32(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

pub fn find_pair_resonance(
    inner_moon_id: Uuid,
    outer_moon_id: Uuid,
    mean_motion_inner: AngularVelocity,
    mean_motion_outer: AngularVelocity,
    max_order: Option<u32>,
    max_deviation: Option<f64>,
) -> Option<MoonPairResonance> {
    let order_limit = max_order.unwrap_or(DEFAULT_MAX_RESONANCE_ORDER);
    let dev_limit = max_deviation.unwrap_or(DEFAULT_MAX_RESONANCE_DEVIATION);

    let (p, q, dev) =
        mean_motion_resonance_search(mean_motion_inner, mean_motion_outer, order_limit)?;

    if dev <= dev_limit {
        Some(MoonPairResonance::new(
            inner_moon_id,
            outer_moon_id,
            p,
            q,
            dev,
        ))
    } else {
        None
    }
}

pub fn find_all_pair_resonances(
    moons: &[(Uuid, AngularVelocity)],
    max_order: Option<u32>,
    max_deviation: Option<f64>,
) -> Vec<MoonPairResonance> {
    let mut resonances = Vec::new();
    let n = moons.len();

    for i in 0..n {
        for j in (i + 1)..n {
            let (id_a, n_a) = moons[i];
            let (id_b, n_b) = moons[j];

            let (inner_id, outer_id, n_inner, n_outer) = if n_a.value() >= n_b.value() {
                (id_a, id_b, n_a, n_b)
            } else {
                (id_b, id_a, n_b, n_a)
            };

            if let Some(res) = find_pair_resonance(
                inner_id,
                outer_id,
                n_inner,
                n_outer,
                max_order,
                max_deviation,
            ) {
                resonances.push(res);
            }
        }
    }

    resonances
}

pub fn find_laplace_trio_resonances(
    pair_resonances: &[MoonPairResonance],
    moons: &[(Uuid, AngularVelocity)],
) -> Vec<LaplaceTrioResonance> {
    let mut trios = Vec::new();
    let moon_map: std::collections::HashMap<Uuid, f64> = moons
        .iter()
        .map(|(id, n)| (*id, n.value()))
        .collect();

    for r1 in pair_resonances {
        for r2 in pair_resonances {
            if r1.outer_moon_id == r2.inner_moon_id && r1.inner_moon_id != r2.outer_moon_id {
                let id1 = r1.inner_moon_id;
                let id2 = r1.outer_moon_id;
                let id3 = r2.outer_moon_id;

                let n1 = match moon_map.get(&id1) {
                    Some(&v) => v,
                    None => continue,
                };
                let n2 = match moon_map.get(&id2) {
                    Some(&v) => v,
                    None => continue,
                };
                let n3 = match moon_map.get(&id3) {
                    Some(&v) => v,
                    None => continue,
                };

                let k1 = r1.p * r2.p;
                let k2 = r1.q * r2.p;
                let k3 = r1.q * r2.q;

                let g = gcd_u32(k1, gcd_u32(k2, k3));
                let ratio1 = if g > 0 { k1 / g } else { k1 };
                let ratio2 = if g > 0 { k2 / g } else { k2 };
                let ratio3 = if g > 0 { k3 / g } else { k3 };

                let target_unit = n3 / (ratio3 as f64);
                if target_unit <= 0.0 {
                    continue;
                }

                let dev1 = ((n1 / (ratio1 as f64)) - target_unit).abs() / target_unit;
                let dev2 = ((n2 / (ratio2 as f64)) - target_unit).abs() / target_unit;
                let total_dev = 0.5 * (dev1 + dev2);

                trios.push(LaplaceTrioResonance::new(
                    id1,
                    id2,
                    id3,
                    ratio1,
                    ratio2,
                    ratio3,
                    total_dev,
                ));
            }
        }
    }

    trios
}
