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
