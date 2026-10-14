use super::{
    Dop853Config, Dop853Error, GravitySource, inertial_point_mass_derivative, solve_dop853,
    solve_dop853_with_event,
};
use crate::math::orbital::propagate_universal_state_vectors;
use astronomicon_core::units::GravitationalParameter;
use astronomicon_core::units::{Duration, Position, Vector3, VelocityVector};
use uuid::Uuid;

fn six_axis_config() -> Dop853Config<6> {
    Dop853Config {
        absolute_tolerances: [0.001, 0.001, 0.001, 1e-6, 1e-6, 1e-6],
        relative_tolerance: 1e-11,
        initial_step_s: 8000.0,
        minimum_step_s: 1e-8,
        maximum_step_s: 10000.0,
        maximum_attempts: 10000,
        maximum_evaluations: 50000,
        maximum_segments: 3000,
    }
}

#[test]
fn temporal_acceleration_and_dense_output_follow_exact_solution() {
    let config = Dop853Config {
        absolute_tolerances: [1e-10, 1e-10],
        relative_tolerance: 1e-12,
        initial_step_s: 3.0,
        minimum_step_s: 1e-10,
        maximum_step_s: 8.0,
        maximum_attempts: 300,
        maximum_evaluations: 1000,
        maximum_segments: 200,
    };
    let solution = solve_dop853(
        [4.0, -2.0],
        20.0,
        config,
        |t, y| Ok([y[1], 2.0 + 0.1 * t]),
        || false,
    )
    .unwrap();
    let expected = |t: f64| {
        [
            4.0 - 2.0 * t + t * t + t * t * t / 60.0,
            -2.0 + 2.0 * t + t * t / 20.0,
        ]
    };
    for segment in &solution.segments {
        for fraction in [0.0, 0.17, 0.37, 0.83, 1.0] {
            let t =
                segment.start_seconds + (segment.end_seconds - segment.start_seconds) * fraction;
            let measured = segment.evaluate(t).unwrap();
            let exact = expected(t);
            assert!((measured[0] - exact[0]).abs() < 1e-8);
            assert!((measured[1] - exact[1]).abs() < 1e-8);
        }
    }
    assert_eq!(solution.reached_seconds, 20.0);
}

#[test]
fn two_body_solution_agrees_with_universal_kepler() {
    let gm = 3.986004418e14;
    let initial = [9_000_000.0, 0.0, 0.0, 0.0, 7000.0, 0.0];
    let f = |_: f64, y: &[f64; 6]| {
        let radius = (y[0] * y[0] + y[1] * y[1] + y[2] * y[2]).sqrt();
        let k = -gm / radius.powi(3);
        Ok([y[3], y[4], y[5], y[0] * k, y[1] * k, y[2] * k])
    };
    let solution = solve_dop853(initial, 11000.0, six_axis_config(), f, || false).unwrap();
    let (p, v) = propagate_universal_state_vectors(
        Position::from_raw(Vector3::new(initial[0], initial[1], initial[2])),
        VelocityVector::from_raw(Vector3::new(initial[3], initial[4], initial[5])),
        GravitationalParameter::new(gm),
        Duration::new(11000.0),
    )
    .unwrap();
    for (a, b) in solution.final_state[0..3]
        .iter()
        .zip([p.raw().0, p.raw().1, p.raw().2])
    {
        assert!((a - b).abs() < 0.2);
    }
    for (a, b) in solution.final_state[3..6]
        .iter()
        .zip([v.raw().0, v.raw().1, v.raw().2])
    {
        assert!((a - b).abs() < 0.0002);
    }
}

fn moving_lunar_force(t: f64, y: &[f64; 6]) -> Result<[f64; 6], Dop853Error> {
    let gm = 3.986004418e14_f64;
    let moon_gm = 4.9048695e12_f64;
    let orbit_radius = 384400000.0_f64;
    let omega = ((gm + moon_gm) / orbit_radius.powi(3)).sqrt();
    let angle = 0.4 + omega * t;
    let moon = [orbit_radius * angle.cos(), orbit_radius * angle.sin(), 0.0];
    let r = [y[0], y[1], y[2]];
    let offset = [moon[0] - r[0], moon[1] - r[1], -r[2]];
    let radius = r.iter().map(|x| x * x).sum::<f64>().sqrt();
    let distance = offset.iter().map(|x| x * x).sum::<f64>().sqrt();
    if radius < 1.0 || distance < 1.0 {
        return Err(Dop853Error::Derivative("gravitational singularity".into()));
    }
    let acceleration = [
        -gm * r[0] / radius.powi(3)
            + moon_gm * (offset[0] / distance.powi(3) - moon[0] / orbit_radius.powi(3)),
        -gm * r[1] / radius.powi(3)
            + moon_gm * (offset[1] / distance.powi(3) - moon[1] / orbit_radius.powi(3)),
        -gm * r[2] / radius.powi(3) + moon_gm * offset[2] / distance.powi(3),
    ];
    Ok([
        y[3],
        y[4],
        y[5],
        acceleration[0],
        acceleration[1],
        acceleration[2],
    ])
}

#[test]
fn moving_third_body_agrees_with_independent_scipy_and_dense_output() {
    let initial = [40_000_000.0, 0.0, 0.0, 0.0, 2800.0, 100.0];
    let solution = solve_dop853(
        initial,
        259200.0,
        six_axis_config(),
        moving_lunar_force,
        || false,
    )
    .unwrap();
    let reference = [
        -5973032.77226689,
        29633197.7038744,
        1058073.06663217,
        -3486.2718417918,
        -1455.60738229088,
        -51.9964543068728,
    ];
    for i in 0..6 {
        let tolerance = if i < 3 { 2.0 } else { 0.0002 };
        assert!(
            (solution.final_state[i] - reference[i]).abs() < tolerance,
            "component {i}: {} versus {}",
            solution.final_state[i],
            reference[i]
        );
    }
    let t = 123456.789;
    let segment = solution
        .segments
        .iter()
        .find(|segment| segment.start_seconds <= t && segment.end_seconds >= t)
        .unwrap();
    let interpolated = segment.evaluate(t).unwrap();
    let reference_mid = [
        37851785.9475121,
        11433036.2716492,
        408345.524741723,
        -1028.78385752394,
        2648.21617610826,
        94.5687608807722,
    ];
    for i in 0..6 {
        let tolerance = if i < 3 { 3.0 } else { 0.0005 };
        assert!(
            (interpolated[i] - reference_mid[i]).abs() < tolerance,
            "dense component {i}: {} versus {}",
            interpolated[i],
            reference_mid[i]
        );
    }
    assert!(solution.evaluations <= six_axis_config().maximum_evaluations);
    assert!(solution.accepted_steps > 2);
}

#[test]
fn fixed_step_refinement_recovers_high_order_convergence() {
    let gm = 3.986004418e14_f64;
    let radius = 7_000_000.0_f64;
    let speed = (gm / radius).sqrt();
    let period = 2.0 * std::f64::consts::PI * (radius.powi(3) / gm).sqrt();
    let initial = [radius, 0.0, 0.0, 0.0, speed, 0.0];
    let simulate = |steps: usize| {
        let h = period / steps as f64;
        let mut config = six_axis_config();
        config.absolute_tolerances = [1e10; 6];
        config.relative_tolerance = 0.0;
        config.initial_step_s = h;
        config.maximum_step_s = h;
        solve_dop853(
            initial,
            period,
            config,
            |_, y| {
                let r = (y[0].powi(2) + y[1].powi(2) + y[2].powi(2)).sqrt();
                let factor = -gm / r.powi(3);
                Ok([
                    y[3],
                    y[4],
                    y[5],
                    y[0] * factor,
                    y[1] * factor,
                    y[2] * factor,
                ])
            },
            || false,
        )
        .unwrap()
    };
    let coarse = simulate(6);
    let fine = simulate(12);
    let norm = |state: [f64; 6]| {
        ((state[0] - radius).powi(2) + state[1].powi(2) + state[2].powi(2)).sqrt()
    };
    let coarse_error = norm(coarse.final_state);
    let fine_error = norm(fine.final_state);
    assert!(
        coarse_error > 40.0 * fine_error,
        "refinement: coarse={coarse_error}, fine={fine_error}"
    );
}

#[test]
fn inertial_force_resolves_each_body_at_the_stage_epoch() {
    let sun_id = Uuid::from_u128(1);
    let companion_id = Uuid::from_u128(2);
    let mut epochs = Vec::new();
    let initial = [40_000_000.0, 0.0, 0.0, 0.0, 2800.0, 0.0];
    let solution = solve_dop853(
        initial,
        86400.0,
        six_axis_config(),
        |time, state| {
            inertial_point_mass_derivative(time, state, |at| {
                epochs.push(at);
                let angle = at * 2e-6;
                Ok([
                    GravitySource {
                        body_id: sun_id,
                        gravitational_parameter_m3_s2: 3.986004418e14,
                        position_m: [0.0; 3],
                    },
                    GravitySource {
                        body_id: companion_id,
                        gravitational_parameter_m3_s2: 4.9048695e12,
                        position_m: [384400000.0 * angle.cos(), 384400000.0 * angle.sin(), 0.0],
                    },
                ])
            })
        },
        || false,
    )
    .unwrap();
    assert_eq!(epochs.len(), solution.evaluations);
    assert!(epochs.iter().any(|time| *time > 0.0 && *time < 86400.0));
    assert!(epochs.iter().any(|time| *time >= 86400.0));
    let duplicate = inertial_point_mass_derivative(0.0, &initial, |_| {
        Ok([
            GravitySource {
                body_id: sun_id,
                gravitational_parameter_m3_s2: 3.986004418e14,
                position_m: [0.0; 3],
            },
            GravitySource {
                body_id: sun_id,
                gravitational_parameter_m3_s2: 4.9048695e12,
                position_m: [384400000.0, 0.0, 0.0],
            },
        ])
    });
    assert!(matches!(duplicate, Err(Dop853Error::InvalidGravitySource)));
}

#[test]
fn event_stops_dense_output_before_the_segment_endpoint() {
    let config = Dop853Config {
        absolute_tolerances: [1e-8],
        relative_tolerance: 1e-12,
        initial_step_s: 10.0,
        minimum_step_s: 1e-6,
        maximum_step_s: 10.0,
        maximum_attempts: 16,
        maximum_evaluations: 256,
        maximum_segments: 16,
    };
    let solution = solve_dop853_with_event(
        [0.0],
        20.0,
        config,
        |_, _| Ok([1.0]),
        || false,
        |segment| {
            Ok(Some(
                segment.start_seconds + 0.25 * (segment.end_seconds - segment.start_seconds),
            ))
        },
    )
    .unwrap();
    let event = solution.event_seconds.unwrap();
    assert!((event - 2.5).abs() < 1e-10);
    assert!((solution.final_state[0] - event).abs() < 1e-9);
    assert!(solution.segments[0].evaluate(event).is_some());
    assert!(solution.segments[0].evaluate(event + 0.1).is_none());
}

#[test]
fn termination_guards_are_explicit() {
    let initial = [4.0, -2.0, 0.0, 0.0, 0.0, 0.0];
    let mut config = six_axis_config();
    config.maximum_evaluations = 16;
    let exhausted = solve_dop853(initial, 100000.0, config, |_, y| Ok(*y), || false);
    assert!(matches!(exhausted, Err(Dop853Error::BudgetExceeded)));
    let mut checks = 0;
    let canceled = solve_dop853(
        initial,
        100000.0,
        six_axis_config(),
        |_, y| Ok(*y),
        || {
            checks += 1;
            checks > 5
        },
    );
    assert!(matches!(canceled, Err(Dop853Error::Cancelled)));
    let nonfinite = solve_dop853(
        initial,
        10.0,
        six_axis_config(),
        |_, _| Ok([f64::NAN; 6]),
        || false,
    );
    assert!(matches!(nonfinite, Err(Dop853Error::NonFiniteDerivative)));
}
