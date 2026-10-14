use crate::{RocketError, RocketResult};
use astronomicon_core::units::Position;

const MAX_DEPTH: u32 = 12;
const RELATIVE_CHORD_ERROR: f64 = 0.003;

fn distance(a: Position, b: Position) -> f64 {
    (a.raw() - b.raw()).magnitude()
}

fn append_interval(
    start: f64,
    end: f64,
    first: Position,
    last: Position,
    depth: u32,
    remaining: &mut usize,
    points: &mut Vec<(f64, Position)>,
    position_at: &mut impl FnMut(f64) -> RocketResult<Position>,
) -> RocketResult<()> {
    let middle = (start + end) * 0.5;
    if middle <= start || middle >= end {
        if *remaining == 0 {
            return Err(RocketError::Generic(
                "forecast geometry point budget exceeded".into(),
            ));
        }
        points.push((end, last));
        *remaining -= 1;
        return Ok(());
    }
    let midpoint = position_at(middle)?;
    let chord_mid = Position::from_raw((first.raw() + last.raw()) * 0.5);
    let chord_length = distance(first, last);
    let error = distance(midpoint, chord_mid);
    if !error.is_finite() || !chord_length.is_finite() {
        return Err(RocketError::Generic(
            "nonfinite forecast curve geometry".into(),
        ));
    }
    if error > RELATIVE_CHORD_ERROR * chord_length.max(1.0) && depth < MAX_DEPTH {
        append_interval(
            start,
            middle,
            first,
            midpoint,
            depth + 1,
            remaining,
            points,
            position_at,
        )?;
        append_interval(
            middle,
            end,
            midpoint,
            last,
            depth + 1,
            remaining,
            points,
            position_at,
        )?;
        return Ok(());
    }
    if *remaining == 0 {
        return Err(RocketError::Generic(
            "forecast geometry point budget exceeded".into(),
        ));
    }
    points.push((end, last));
    *remaining -= 1;
    Ok(())
}

pub(super) fn sample_smooth_curve(
    start: f64,
    end: f64,
    first: Position,
    last: Position,
    maximum_points: usize,
    mut position_at: impl FnMut(f64) -> RocketResult<Position>,
) -> RocketResult<Vec<(f64, Position)>> {
    if !start.is_finite() || !end.is_finite() || end <= start || maximum_points == 0 {
        return Err(RocketError::Generic(
            "invalid forecast geometry sampling interval".into(),
        ));
    }
    let mut points = Vec::new();
    let mut remaining = maximum_points;
    append_interval(
        start,
        end,
        first,
        last,
        0,
        &mut remaining,
        &mut points,
        &mut position_at,
    )?;
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use astronomicon_core::units::Vector3;

    #[test]
    fn curved_flyby_is_refined_beyond_its_endpoints() {
        let point = |t: f64| Position::from_raw(Vector3::new(t.cos(), t.sin(), 0.0));
        let samples = sample_smooth_curve(
            0.0,
            std::f64::consts::PI,
            point(0.0),
            point(std::f64::consts::PI),
            128,
            |t| Ok(point(t)),
        )
        .unwrap();
        assert!(samples.len() > 8);
        assert!((samples.last().unwrap().1.raw().0 + 1.0).abs() < 1e-12);
        for pair in samples.windows(2) {
            assert!(pair[1].0 > pair[0].0);
        }
    }

    #[test]
    fn straight_trajectory_does_not_generate_spurious_vertices() {
        let point = |t: f64| Position::from_raw(Vector3::new(t, 2.0 * t, 0.0));
        let samples =
            sample_smooth_curve(0.0, 100.0, point(0.0), point(100.0), 8, |t| Ok(point(t))).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].0, 100.0);
    }

    #[test]
    fn insufficient_budget_fails_instead_of_publishing_a_false_line() {
        let point = |t: f64| Position::from_raw(Vector3::new(t.cos(), t.sin(), 0.0));
        assert!(
            sample_smooth_curve(
                0.0,
                std::f64::consts::PI,
                point(0.0),
                point(std::f64::consts::PI),
                2,
                |t| Ok(point(t)),
            )
            .is_err()
        );
    }
}
