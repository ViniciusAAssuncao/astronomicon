use super::dop853::{Dop853DenseSegment, Dop853Error};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactCandidate {
    pub body_id: Uuid,
    pub radius_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactEvent {
    pub body_id: Uuid,
    pub elapsed_seconds: f64,
    pub radial_residual_m: f64,
}

fn refine_entrance(
    mut outside: f64,
    mut contact: f64,
    accuracy_s: f64,
    clearance: &mut impl FnMut(f64) -> Result<f64, Dop853Error>,
) -> Result<(f64, f64), Dop853Error> {
    for _ in 0..80 {
        if contact - outside <= accuracy_s {
            break;
        }
        let mid = outside + (contact - outside) * 0.5;
        if mid == outside || mid == contact {
            break;
        }
        if clearance(mid)? <= 0.0 {
            contact = mid;
        } else {
            outside = mid;
        }
    }
    Ok((contact, clearance(contact)?.abs()))
}

fn refine_minimum(
    mut left: f64,
    mut right: f64,
    accuracy_s: f64,
    clearance: &mut impl FnMut(f64) -> Result<f64, Dop853Error>,
) -> Result<(f64, f64), Dop853Error> {
    let factor = 0.3819660112501051;
    let mut a = left + (right - left) * factor;
    let mut b = right - (right - left) * factor;
    let mut fa = clearance(a)?;
    let mut fb = clearance(b)?;
    for _ in 0..80 {
        if right - left <= accuracy_s {
            break;
        }
        if fa <= fb {
            right = b;
            b = a;
            fb = fa;
            a = left + (right - left) * factor;
            if a == left || a == b {
                break;
            }
            fa = clearance(a)?;
        } else {
            left = a;
            a = b;
            fa = fb;
            b = right - (right - left) * factor;
            if b == a || b == right {
                break;
            }
            fb = clearance(b)?;
        }
    }
    let mid = left + (right - left) * 0.5;
    let fm = clearance(mid)?;
    if fa <= fb && fa <= fm {
        Ok((a, fa))
    } else if fb <= fm {
        Ok((b, fb))
    } else {
        Ok((mid, fm))
    }
}

pub fn first_impact_in_segment<const N: usize>(
    segment: &Dop853DenseSegment<N>,
    candidates: &[ImpactCandidate],
    accuracy_s: f64,
    samples_per_body: usize,
    mut body_position: impl FnMut(Uuid, f64) -> Result<[f64; 3], Dop853Error>,
) -> Result<Option<ImpactEvent>, Dop853Error> {
    let start = segment.start_seconds;
    let end = segment.valid_until_seconds;
    if N < 3
        || !start.is_finite()
        || !end.is_finite()
        || end <= start
        || !accuracy_s.is_finite()
        || accuracy_s <= 0.0
        || !(8..=512).contains(&samples_per_body)
    {
        return Err(Dop853Error::InvalidConfiguration);
    }
    let mut earliest: Option<ImpactEvent> = None;
    let mut evaluations = 0usize;
    for candidate in candidates {
        if !candidate.radius_m.is_finite() || candidate.radius_m <= 0.0 {
            return Err(Dop853Error::InvalidGravitySource);
        }
        let mut clearance = |t: f64| -> Result<f64, Dop853Error> {
            let craft = segment.evaluate(t).ok_or(Dop853Error::NumericalBreakdown)?;
            if evaluations >= 4096 {
                return Err(Dop853Error::BudgetExceeded);
            }
            evaluations += 1;
            let body = body_position(candidate.body_id, t)?;
            if !body.iter().all(|v| v.is_finite()) {
                return Err(Dop853Error::NonFiniteDerivative);
            }
            let distance = (0..3)
                .map(|i| (craft[i] - body[i]).powi(2))
                .sum::<f64>()
                .sqrt();
            if !distance.is_finite() {
                return Err(Dop853Error::NumericalBreakdown);
            }
            Ok(distance - candidate.radius_m)
        };
        let mut samples = Vec::with_capacity(samples_per_body + 1);
        for index in 0..=samples_per_body {
            let t = if index == samples_per_body {
                end
            } else {
                start + (end - start) * index as f64 / samples_per_body as f64
            };
            samples.push((t, clearance(t)?));
        }
        let mut first = None;
        if samples[0].1 <= 0.0 {
            first = Some((start, samples[0].1.abs()));
        } else {
            let interval = samples[1].0 - samples[0].0;
            let probe = (interval * 1e-4).max(accuracy_s).min(interval * 0.125);
            if samples[1].1 > samples[0].1 && clearance(start + probe)? < samples[0].1 {
                let (minimum_t, minimum) =
                    refine_minimum(start, samples[1].0, accuracy_s, &mut clearance)?;
                if minimum <= 0.0 {
                    first = Some(if minimum == 0.0 {
                        (minimum_t, 0.0)
                    } else {
                        refine_entrance(start, minimum_t, accuracy_s, &mut clearance)?
                    });
                }
            }
            for index in 1..samples.len() {
                if first.is_some() {
                    break;
                }
                let (before_t, before) = samples[index - 1];
                let (after_t, after) = samples[index];
                if before > 0.0 && after <= 0.0 {
                    let entrance = refine_entrance(before_t, after_t, accuracy_s, &mut clearance)?;
                    first = Some(entrance);
                    break;
                }
                if index + 1 < samples.len() && after < before && after <= samples[index + 1].1 {
                    let (minimum_t, minimum) =
                        refine_minimum(before_t, samples[index + 1].0, accuracy_s, &mut clearance)?;
                    if minimum <= 0.0 {
                        let entrance = if minimum == 0.0 {
                            (minimum_t, minimum.abs())
                        } else {
                            refine_entrance(before_t, minimum_t, accuracy_s, &mut clearance)?
                        };
                        first = Some(entrance);
                        break;
                    }
                }
            }
            if first.is_none() {
                let last = samples.len() - 1;
                let interval = samples[last].0 - samples[last - 1].0;
                let probe = (interval * 1e-4).max(accuracy_s).min(interval * 0.125);
                if samples[last - 1].1 > samples[last].1
                    && clearance(end - probe)? < samples[last].1
                {
                    let (minimum_t, minimum) =
                        refine_minimum(samples[last - 1].0, end, accuracy_s, &mut clearance)?;
                    if minimum <= 0.0 {
                        first = Some(if minimum == 0.0 {
                            (minimum_t, 0.0)
                        } else {
                            refine_entrance(
                                samples[last - 1].0,
                                minimum_t,
                                accuracy_s,
                                &mut clearance,
                            )?
                        });
                    }
                }
            }
        }
        if let Some((time, residual)) = first {
            if earliest.is_none_or(|previous| time < previous.elapsed_seconds) {
                earliest = Some(ImpactEvent {
                    body_id: candidate.body_id,
                    elapsed_seconds: time,
                    radial_residual_m: residual,
                });
            }
        }
    }
    Ok(earliest)
}