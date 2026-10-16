use super::orbit_soi::{OrbitSoiPreview, outside_soi, refine_exit};
use astronomicon_core::units::{Duration, GravitationalParameter, Position, VelocityVector};
use rocketcon_core::math::orbital::propagate_universal_state_vectors;

const MAX_POINTS: usize = 257;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewEndReason {
    Horizon,
    Impact,
    SoiExit,
    OutsideSoi,
    NumericalLimit,
    SampleLimit,
}

pub struct PreviewPath {
    pub points: Vec<[f64; 3]>,
    pub elapsed_seconds: f64,
    pub impact_seconds: Option<f64>,
    pub end_reason: PreviewEndReason,
}

pub fn sample_preview_path(
    position: Position,
    velocity: VelocityVector,
    mu: GravitationalParameter,
    body_radius_m: f64,
    horizon_seconds: f64,
    soi: &mut Option<OrbitSoiPreview>,
) -> PreviewPath {
    let mut path = PreviewPath {
        points: Vec::with_capacity(MAX_POINTS),
        elapsed_seconds: 0.0,
        impact_seconds: None,
        end_reason: PreviewEndReason::Horizon,
    };
    if soi.as_ref().is_some_and(|s| s.outside) {
        path.end_reason = PreviewEndReason::OutsideSoi;
        return path;
    }
    let (mut r, mut v) = (position, velocity);
    let mut time = 0.0;
    let angular_momentum = position.raw().cross(&velocity.raw()).magnitude();
    let max_step = (horizon_seconds / 72.0).max(1.0);
    for _ in 0..MAX_POINTS {
        let raw = r.raw();
        if ![raw.0, raw.1, raw.2].iter().all(|x| x.is_finite()) {
            path.end_reason = PreviewEndReason::NumericalLimit;
            break;
        }
        path.points.push([raw.0, raw.1, raw.2]);
        path.elapsed_seconds = time;
        if body_radius_m > 0.0 && raw.magnitude() <= body_radius_m {
            path.impact_seconds = Some(time);
            path.end_reason = PreviewEndReason::Impact;
            break;
        }
        if time >= horizon_seconds - 1e-7 {
            break;
        }
        let radius = raw.magnitude();
        let angular_step = if angular_momentum > 1e-8 {
            (0.075 * radius * radius / angular_momentum).max(1.0)
        } else {
            max_step
        };
        let mut step = (horizon_seconds - time).min(angular_step).min(max_step);
        let mut next = None;
        for _ in 0..8 {
            if let Ok(state) = propagate_universal_state_vectors(r, v, mu, Duration::new(step)) {
                next = Some(state);
                break;
            }
            step *= 0.5;
        }
        let Some((next_r, next_v)) = next else {
            path.end_reason = PreviewEndReason::NumericalLimit;
            break;
        };
        if let Some(boundary) = soi.as_mut() {
            if !outside_soi(r, boundary.radius_m) && outside_soi(next_r, boundary.radius_m) {
                if let Some(mut event) = refine_exit(r, v, mu, step, boundary.radius_m) {
                    event.time_to_seconds += time;
                    path.points.push(event.relative_position_m);
                    path.elapsed_seconds = event.time_to_seconds;
                    boundary.exit = Some(event);
                    path.end_reason = PreviewEndReason::SoiExit;
                } else {
                    path.end_reason = PreviewEndReason::NumericalLimit;
                }
                break;
            }
        }
        if body_radius_m > 0.0 && next_r.raw().magnitude() <= body_radius_m {
            let (mut low, mut high) = (0.0, step);
            for _ in 0..24 {
                let mid = (low + high) * 0.5;
                match propagate_universal_state_vectors(r, v, mu, Duration::new(mid)) {
                    Ok((state, _)) if state.raw().magnitude() <= body_radius_m => high = mid,
                    Ok(_) => low = mid,
                    Err(_) => {
                        path.end_reason = PreviewEndReason::NumericalLimit;
                        return path;
                    }
                }
            }
            if let Ok((impact, _)) =
                propagate_universal_state_vectors(r, v, mu, Duration::new(high))
            {
                let point = impact.raw();
                path.points.push([point.0, point.1, point.2]);
                path.elapsed_seconds = time + high;
                path.impact_seconds = Some(time + high);
                path.end_reason = PreviewEndReason::Impact;
            } else {
                path.end_reason = PreviewEndReason::NumericalLimit;
            }
            break;
        }
        time += step;
        (r, v) = (next_r, next_v);
    }
    if path.end_reason == PreviewEndReason::Horizon && path.elapsed_seconds + 1e-6 < horizon_seconds
    {
        path.end_reason = PreviewEndReason::SampleLimit;
    }
    path
}
