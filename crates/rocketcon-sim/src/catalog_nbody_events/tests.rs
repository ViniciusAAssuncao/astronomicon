use super::*;
use rocketcon_core::math::orbital::n_body::dop853::{Dop853Config, solve_dop853};

fn segment() -> Dop853DenseSegment<6> {
    let settings = Dop853Config {
        absolute_tolerances: [1e-8; 6],
        relative_tolerance: 1e-12,
        initial_step_s: 10.0,
        minimum_step_s: 1e-6,
        maximum_step_s: 10.0,
        maximum_attempts: 64,
        maximum_evaluations: 256,
        maximum_segments: 16,
    };
    solve_dop853(
        [-10.0, 0.0, 0.0, 4.0, 0.0, 0.0],
        10.0,
        settings,
        |_, y| Ok([y[3], y[4], y[5], 0.0, 0.0, 0.0]),
        || false,
    )
    .unwrap()
    .segments
    .remove(0)
}

fn body() -> ImpactCandidate {
    ImpactCandidate {
        body_id: Uuid::from_u128(1),
        radius_m: 1.0,
    }
}

#[test]
fn outside_outside_crossing_is_located() {
    let event = first_impact_in_segment(&segment(), &[body()], 1e-9, 8, |_, _| Ok([0.0; 3]))
        .unwrap()
        .unwrap();
    assert!((event.elapsed_seconds - 2.25).abs() < 1e-7);
    assert!(event.radial_residual_m < 1e-7);
}

#[test]
fn moving_target_is_evaluated_at_event_epoch() {
    let event = first_impact_in_segment(&segment(), &[body()], 1e-9, 8, |_, t| {
        Ok([0.5 * t, 0.0, 0.0])
    })
    .unwrap()
    .unwrap();
    assert!((event.elapsed_seconds - 9.0 / 3.5).abs() < 1e-7);
}

#[test]
fn narrow_crossing_between_coarse_samples_is_detected() {
    let event = first_impact_in_segment(
        &segment(),
        &[ImpactCandidate {
            body_id: body().body_id,
            radius_m: 0.04,
        }],
        1e-9,
        8,
        |_, _| Ok([-0.52, 0.0, 0.0]),
    )
    .unwrap()
    .unwrap();
    assert!((event.elapsed_seconds - 2.36).abs() < 1e-7);
}

#[test]
fn exact_tangency_is_detected_without_negative_samples() {
    let event =
        first_impact_in_segment(&segment(), &[body()], 1e-9, 8, |_, _| Ok([-0.52, 1.0, 0.0]))
            .unwrap()
            .unwrap();
    assert!((event.elapsed_seconds - 2.37).abs() < 1e-6);
    assert!(event.radial_residual_m < 1e-8);
}

#[test]
fn near_tangent_passage_is_not_reported_as_collision() {
    let event = first_impact_in_segment(&segment(), &[body()], 1e-9, 8, |_, _| {
        Ok([-0.52, 1.001, 0.0])
    })
    .unwrap();
    assert!(event.is_none());
}

#[test]
fn first_event_is_chosen_across_bodies() {
    let candidates = [
        body(),
        ImpactCandidate {
            body_id: Uuid::from_u128(2),
            radius_m: 1.0,
        },
    ];
    let event = first_impact_in_segment(&segment(), &candidates, 1e-9, 8, |id, _| {
        Ok(if id == body().body_id {
            [4.0, 0.0, 0.0]
        } else {
            [0.0, 0.0, 0.0]
        })
    })
    .unwrap()
    .unwrap();
    assert_eq!(event.body_id, Uuid::from_u128(2));
    assert!((event.elapsed_seconds - 2.25).abs() < 1e-7);
}

#[test]
fn no_contact_is_not_reported() {
    let event =
        first_impact_in_segment(&segment(), &[body()], 1e-9, 8, |_, _| Ok([0.0, 50.0, 0.0]))
            .unwrap();
    assert!(event.is_none());
}

#[test]
fn excessive_event_sampling_fails_explicitly() {
    let candidates: Vec<_> = (1..=12)
        .map(|id| ImpactCandidate {
            body_id: Uuid::from_u128(id),
            radius_m: 1.0,
        })
        .collect();
    let result = first_impact_in_segment(&segment(), &candidates, 1e-9, 512, |_, _| {
        Ok([0.0, 50.0, 0.0])
    });
    assert!(matches!(result, Err(Dop853Error::BudgetExceeded)));
}

#[test]
fn tangent_inside_last_coarse_interval_is_detected() {
    let event =
        first_impact_in_segment(&segment(), &[body()], 1e-9, 8, |_, _| Ok([29.8, 1.0, 0.0]))
            .unwrap()
            .unwrap();
    assert!((event.elapsed_seconds - 9.95).abs() < 1e-6);
}

#[test]
fn tangent_inside_first_coarse_interval_is_detected() {
    let event =
        first_impact_in_segment(&segment(), &[body()], 1e-9, 8, |_, _| Ok([-9.8, 1.0, 0.0]))
            .unwrap();
    assert!(event.is_some());
    assert!((event.unwrap().elapsed_seconds - 0.05).abs() < 1e-6);
}
