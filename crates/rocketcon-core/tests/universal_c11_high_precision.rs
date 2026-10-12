use astronomicon_core::units::{
    Duration, GravitationalParameter, Position, Vector3, VelocityVector,
};
use rocketcon_core::math::orbital::universal::{
    propagate_universal_state_vectors, solve_universal_kepler,
};

#[test]
fn near_radial_reference_states_away_from_point_singularity() {
    let mu: f64 = 3.986_004_418e14;
    let r0: f64 = 7e6;
    let vc = (mu / r0).sqrt();
    let dt = std::f64::consts::PI / (2.0 * 2.0_f64.sqrt()) * (r0.powi(3) / mu).sqrt();
    let cases = [
        (
            1e-3,
            1012.6500109314582,
            119.26455909815138,
            -882633.8476207827,
            -51789.42683524498,
            1e-9,
        ),
        (
            1e-4,
            47.386143656977254,
            2.5766092324417817,
            -4097094.5533362227,
            -111306.25837414303,
            1e-7,
        ),
    ];
    for (ratio, x, y, vx, vy, tolerance) in cases {
        let (p, v) = propagate_universal_state_vectors(
            Position::from_raw(Vector3::new(r0, 0.0, 0.0)),
            VelocityVector::from_raw(Vector3::new(0.0, ratio * vc, 0.0)),
            GravitationalParameter::new(mu),
            Duration::new(dt),
        )
        .expect("point-mass Kepler propagation should be defined at this state");
        let expected_p = Vector3::new(x, y, 0.0);
        let expected_v = Vector3::new(vx, vy, 0.0);
        let rel_p = (p.raw() - expected_p).magnitude() / expected_p.magnitude();
        let rel_v = (v.raw() - expected_v).magnitude() / expected_v.magnitude();
        assert!(
            rel_p < tolerance,
            "near-radial {ratio}: position error {rel_p}"
        );
        assert!(
            rel_v < tolerance,
            "near-radial {ratio}: velocity error {rel_v}"
        );
    }
}
#[test]
fn rc1_compensated_pericenter_matches_independent_cartesian_reference() {
    let mu: f64 = 3.986_004_418e14;
    let r0: f64 = 7e6;
    let dt = 1030.3459097688751_f64;
    let vt = 1e-5 * (mu / r0).sqrt();
    let (p, v) = propagate_universal_state_vectors(
        Position::from_raw(Vector3::new(r0, 0.0, 0.0)),
        VelocityVector::from_raw(Vector3::new(0.0, vt, 0.0)),
        GravitationalParameter::new(mu),
        Duration::new(dt),
    )
    .expect("RC1 near-pericenter state");
    let expected_p = Vector3::new(-0.0003324603821532397, 0.00015670183473687576, 0.0);
    let expected_v = Vector3::new(-321728689.3866616, -1437188549.5003536, 0.0);
    let relative_p = (p.raw() - expected_p).magnitude() / expected_p.magnitude();
    let relative_v = (v.raw() - expected_v).magnitude() / expected_v.magnitude();
    assert!(relative_p < 1e-9, "RC1 position error {relative_p:e}");
    assert!(relative_v < 1e-9, "RC1 velocity error {relative_v:e}");
    let (rotated_p, rotated_v) = propagate_universal_state_vectors(
        Position::from_raw(Vector3::new(0.0, 0.0, r0)),
        VelocityVector::from_raw(Vector3::new(vt, 0.0, 0.0)),
        GravitationalParameter::new(mu),
        Duration::new(dt),
    )
    .expect("critical axis-permuted state");
    let expected_rotated_p = Vector3::new(expected_p.1, 0.0, expected_p.0);
    let expected_rotated_v = Vector3::new(expected_v.1, 0.0, expected_v.0);
    assert!(
        (rotated_p.raw() - expected_rotated_p).magnitude() / expected_rotated_p.magnitude() < 1e-9
    );
    assert!(
        (rotated_v.raw() - expected_rotated_v).magnitude() / expected_rotated_v.magnitude() < 1e-9
    );
}
#[test]
fn rc1_backward_critical_state_obeys_time_reversal() {
    let mu: f64 = 3.986_004_418e14;
    let r0: f64 = 7e6;
    let position = Vector3::new(r0, 0.0, 0.0);
    let velocity = Vector3::new(0.0, 1e-5 * (mu / r0).sqrt(), 0.0);
    let dt = 1030.3459097688751_f64;
    let (forward_p, forward_v) = propagate_universal_state_vectors(
        Position::from_raw(position),
        VelocityVector::from_raw(velocity),
        GravitationalParameter::new(mu),
        Duration::new(dt),
    )
    .expect("forward critical state");
    let (backward_p, backward_v) = propagate_universal_state_vectors(
        Position::from_raw(position),
        VelocityVector::from_raw(velocity * -1.0),
        GravitationalParameter::new(mu),
        Duration::new(-dt),
    )
    .expect("backward critical state");
    assert_eq!(forward_p.raw(), backward_p.raw());
    assert_eq!(forward_v.raw(), backward_v.raw() * -1.0);
    let forward_chi = solve_universal_kepler(position, velocity, mu, dt).expect("forward anomaly");
    let backward_chi =
        solve_universal_kepler(position, velocity * -1.0, mu, -dt).expect("backward anomaly");
    assert_eq!(forward_chi, -backward_chi);
}
