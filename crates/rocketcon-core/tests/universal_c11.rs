use astronomicon_core::units::{
    Duration, GravitationalParameter, Position, Vector3, VelocityVector,
};

use rocketcon_core::math::orbital::universal::{
    propagate_universal_state_vectors, solve_universal_kepler, stumpff_c, stumpff_s,
};
fn propagate(r: Vector3, v: Vector3, mu: f64, dt: f64) -> (Vector3, Vector3) {
    let (position, velocity) = propagate_universal_state_vectors(
        Position::from_raw(r),
        VelocityVector::from_raw(v),
        GravitationalParameter::new(mu),
        Duration::new(dt),
    )
    .unwrap();
    (position.raw(), velocity.raw())
}
fn assert_conserved(r: Vector3, v: Vector3, nr: Vector3, nv: Vector3, mu: f64) {
    let e0 = v.dot(&v) / 2.0 - mu / r.magnitude();
    let e1 = nv.dot(&nv) / 2.0 - mu / nr.magnitude();
    let h0 = r.cross(&v).magnitude();
    let h1 = nr.cross(&nv).magnitude();
    let energy_scale = v.dot(&v) / 2.0 + mu / r.magnitude();
    assert!(
        (e1 - e0).abs() / energy_scale < 2e-8,
        "energy relative error {}",
        (e1 - e0).abs() / energy_scale
    );
    assert!(
        (h1 - h0).abs() / h0.max(1.0) < 2e-8,
        "angular momentum error"
    );
}
#[test]
fn orbital_regimes_conserve_energy_momentum_and_reverse() {
    let mu: f64 = 3.986_004_418e14;
    let radius: f64 = 7.0e6;
    let circular = (mu / radius).sqrt();
    for (label, v, dt) in [
        ("circular", Vector3::new(0.0, circular, 0.0), 5200.0),
        ("elliptic", Vector3::new(450.0, 6500.0, 900.0), 7500.0),
        ("hyperbolic", Vector3::new(-1800.0, 13000.0, 300.0), 15000.0),
        (
            "near_parabolic",
            Vector3::new(300.0, (2.0 * mu / radius).sqrt() * (1.0 + 1e-8), 0.0),
            15000.0,
        ),
        ("retrograde", Vector3::new(0.0, -circular, 100.0), 5200.0),
    ] {
        let r = Vector3::new(radius, 0.0, 0.0);
        let (p, w) = propagate(r, v, mu, dt);
        assert_conserved(r, v, p, w, mu);
        let (rback, vback) = propagate(p, w, mu, -dt);
        assert!(
            (rback - r).magnitude() / radius < 2e-8,
            "{label}: reverse r"
        );
        assert!(
            (vback - v).magnitude() / v.magnitude() < 2e-8,
            "{label}: reverse v"
        );
    }
}
#[test]
fn long_hyperbolic_propagation_is_reversible_and_finite() {
    let mu = 3.8667e13;
    let r = Vector3::new(3_100_000.0, 0.0, 0.0);
    let v = Vector3::new(500.0, 9_000.0, 100.0);
    let (p, w) = propagate(r, v, mu, 1_000_000.0);
    assert!(p.magnitude().is_finite() && w.magnitude().is_finite());
    assert_conserved(r, v, p, w, mu);
    let (rp, vp) = propagate(p, w, mu, -1_000_000.0);
    assert!((rp - r).magnitude() / r.magnitude() < 1e-7);
    assert!((vp - v).magnitude() / v.magnitude() < 1e-7);
}
#[test]
fn near_parabolic_and_radial_limits_preserve_state() {
    let mu = 3.986_004_418e14;
    let r = Vector3::new(7.0e6, 0.0, 0.0);
    let escape = (2.0 * mu / r.magnitude()).sqrt();
    for velocity in [
        Vector3::new(0.0, escape, 0.0),
        Vector3::new(0.0, escape * (1.0 - 1e-9), 0.0),
        Vector3::new(0.0, escape * (1.0 + 1e-9), 0.0),
        Vector3::new(12000.0, 0.0, 0.0),
    ] {
        let (p, w) = propagate(r, velocity, mu, 4000.0);
        let (rp, vp) = propagate(p, w, mu, -4000.0);
        assert!((rp - r).magnitude() / r.magnitude() < 1e-7);
        assert!((vp - velocity).magnitude() / velocity.magnitude() < 1e-7);
        if r.cross(&velocity).magnitude() > 1.0 {
            assert_conserved(r, velocity, p, w, mu);
        }
    }
}

#[test]
fn historical_c9_inbound_hyperbolic_states_resolve() {
    let radius = 142_973_122.607 - 1_500_000.0;
    let mu = 3.8667e13;
    for radial in [-4_000.0, -200.0] {
        let r = Vector3::new(radius, 0.0, 0.0);
        let v = Vector3::new(radial, 300.0, 0.0);
        for dt in [3600.0, 20_000.0, 259_200.0] {
            let root = solve_universal_kepler(r, v, mu, dt).unwrap();
            assert!(root.is_finite());
            let (p, w) = propagate(r, v, mu, dt);
            assert!(p.magnitude().is_finite() && w.magnitude().is_finite());
            assert_conserved(r, v, p, w, mu);
        }
    }
}

#[test]
fn circular_orbit_matches_independent_trigonometric_solution() {
    let mu: f64 = 3.986_004_418e14;
    let radius: f64 = 7.0e6;
    let speed = (mu / radius).sqrt();
    let r = Vector3::new(radius, 0.0, 0.0);
    let v = Vector3::new(0.0, speed, 0.0);
    for dt in [-9_000.0, 1_200.0, 1_000_000.0, 10_000_000_000.0] {
        let angle = speed / radius * dt;
        let expected_r = Vector3::new(radius * angle.cos(), radius * angle.sin(), 0.0);
        let expected_v = Vector3::new(-speed * angle.sin(), speed * angle.cos(), 0.0);
        let (new_r, new_v) = propagate(r, v, mu, dt);
        assert!((new_r - expected_r).magnitude() / radius < 2e-8);
        assert!((new_v - expected_v).magnitude() / speed < 2e-8);
    }
}

#[test]
fn lambert_reference_still_uses_valid_stumpff_values() {
    use rocketcon_core::math::orbital::lambert_solver::{solve_lambert, TransferDirection};
    let r1 = Position::from_raw(Vector3::new(5_000_000.0, 10_000_000.0, 2_100_000.0));
    let r2 = Position::from_raw(Vector3::new(-14_600_000.0, 2_500_000.0, 7_000_000.0));
    let mu = GravitationalParameter::new(3.986_004_418e14);
    let solution = solve_lambert(
        r1,
        r2,
        Duration::new(3600.0),
        mu,
        TransferDirection::ShortWay,
    )
    .unwrap();
    let expected = Vector3::new(-5992.495, 1925.367, 3245.638);
    assert!((solution.departure_velocity.raw() - expected).magnitude() < 2.0);
}

#[test]
fn near_parabolic_extremely_long_horizon_is_finite() {
    let mu: f64 = 3.986_004_418e14;
    let r = Vector3::new(7_000_000.0, 0.0, 0.0);
    let escape = (2.0 * mu / r.magnitude()).sqrt();
    for factor in [1.0 - 1e-8, 1.0 + 1e-8] {
        let v = Vector3::new(1.0, escape * factor, 0.0);
        for dt in [-10_000_000_000.0, 10_000_000_000.0] {
            let (p, w) = propagate(r, v, mu, dt);
            assert!(p.magnitude().is_finite());
            assert_conserved(r, v, p, w, mu);
        }
    }
}

#[test]
fn parabolic_barker_solution_matches_universal_state() {
    let mu: f64 = 3.986_004_418e14;
    let rp: f64 = 7_000_000.0;
    let vp = (2.0 * mu / rp).sqrt();
    let tau = (2.0 * rp.powi(3) / mu).sqrt();
    let r = Vector3::new(rp, 0.0, 0.0);
    let v = Vector3::new(0.0, vp, 0.0);
    for dt in [-1e10, -1e6, -1000.0, 0.0, 1000.0, 1e6, 1e10] {
        let d = 2.0 * ((1.5 * dt / tau).asinh() / 3.0).sinh();
        let pr = Vector3::new(rp * (1.0 - d * d), 2.0 * rp * d, 0.0);
        let pv = Vector3::new(-vp * d / (1.0 + d * d), vp / (1.0 + d * d), 0.0);
        let (r1, v1) = propagate(r, v, mu, dt);
        assert!(
            (r1 - pr).magnitude() / pr.magnitude() < 5e-9,
            "Barker r dt={dt}"
        );
        assert!(
            (v1 - pv).magnitude() / pv.magnitude() < 5e-9,
            "Barker v dt={dt}"
        );
    }
}

#[test]
fn stumpff_small_arguments_agree_with_high_order_series() {
    for z in [-0.1, -1e-5, 0.0, 1e-5, 0.1] {
        let mut c = 0.5;
        let mut s = 1.0 / 6.0;
        let mut term_c = 0.5;
        let mut term_s = 1.0 / 6.0;
        for k in 1..=12 {
            term_c *= -z / ((2 * k + 1) * (2 * k + 2)) as f64;
            term_s *= -z / ((2 * k + 2) * (2 * k + 3)) as f64;
            c += term_c;
            s += term_s;
        }
        assert!((stumpff_c(z) - c).abs() < 5e-15);
        assert!((stumpff_s(z) - s).abs() < 5e-15);
    }
}

#[test]
fn deterministic_sweep_of_scaled_orbit_regimes() {
    let mut cases = 0;
    for radius in [1e5_f64, 7e6, 1e9, 1e11] {
        for mu in [1e12_f64, 3.986004418e14, 1e20] {
            let vc = (mu / radius).sqrt();
            let tau = (radius.powi(3) / mu).sqrt();
            for ratio in [0.55_f64, 0.99, 1.0, 1.2, 2_f64.sqrt() * (1.0 + 1e-8), 2.5] {
                for (direction, tscale) in [(-1.0, 0.01), (1.0, 1.0), (1.0, 20.0)] {
                    let r = Vector3::new(radius, 0.0, 0.0);
                    let v = Vector3::new(0.1 * vc, ratio * vc, 0.02 * vc);
                    let (p, w) = propagate(r, v, mu, direction * tscale * tau);
                    assert_conserved(r, v, p, w, mu);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 216);
}

#[test]
fn coherent_unit_rescaling_preserves_two_body_propagation() {
    let base_r = Vector3::new(7.0e6, 0.0, 0.0);
    let base_v = Vector3::new(600.0, 8_200.0, -100.0);
    let base_mu = 3.986004418e14;
    let base_dt = 4_500.0;
    let (expected_r, expected_v) = propagate(base_r, base_v, base_mu, base_dt);
    for (length_scale, time_scale) in [
        (1e-14_f64, 1e-10_f64),
        (1e-8_f64, 1e-2_f64),
        (1e-6, 1e-3),
        (1e-3, 1.0),
        (1.0, 1.0),
        (1e6, 1e3),
        (1e9, 1e5),
    ] {
        let velocity_scale = length_scale / time_scale;
        let transformed_mu = base_mu * length_scale.powi(3) / time_scale.powi(2);
        let (r, v) = propagate(
            base_r * length_scale,
            base_v * velocity_scale,
            transformed_mu,
            base_dt * time_scale,
        );
        let pos_error = (r / length_scale - expected_r).magnitude() / expected_r.magnitude();
        let vel_error = (v / velocity_scale - expected_v).magnitude() / expected_v.magnitude();
        assert!(
            pos_error < 2e-9,
            "scale {length_scale}/{time_scale}: r {pos_error}"
        );
        assert!(
            vel_error < 2e-9,
            "scale {length_scale}/{time_scale}: v {vel_error}"
        );
    }
}

#[test]
fn lagrange_coefficients_preserve_symplectic_identity_and_radius() {
    let mu: f64 = 3.986_004_418e14;
    let r = Vector3::new(7.0e6, 0.0, 0.0);
    for (v, dt) in [
        (Vector3::new(0.0, 7600.0, 0.0), 4500.0),
        (Vector3::new(-1200.0, 11800.0, 200.0), 20000.0),
        (Vector3::new(300.0, 9000.0, -100.0), -10000.0),
        (
            Vector3::new(0.0, (2.0 * mu / r.magnitude()).sqrt(), 0.0),
            1e7,
        ),
    ] {
        let chi = solve_universal_kepler(r, v, mu, dt).unwrap();
        let r0 = r.magnitude();
        let alpha = 2.0 / r0 - v.dot(&v) / mu;
        let z = alpha * chi * chi;
        let c = stumpff_c(z);
        let s = stumpff_s(z);
        let sigma = r.dot(&v) / mu.sqrt();
        let f = 1.0 - chi * chi * c / r0;
        let g = dt - chi * chi * chi * s / mu.sqrt();
        let propagated_radius =
            sigma * chi * (1.0 - z * s) + (1.0 - alpha * r0) * chi * chi * c + r0;
        let fdot = mu.sqrt() * chi * (z * s - 1.0) / (propagated_radius * r0);
        let gdot = 1.0 - chi * chi * c / propagated_radius;
        let (position, velocity) = propagate(r, v, mu, dt);
        let det = f * gdot - fdot * g;
        assert!(
            (det - 1.0).abs() < 2e-8,
            "lagrange determinant dt={dt} det={det}"
        );
        assert!(
            (position.magnitude() - propagated_radius).abs() / position.magnitude() < 2e-8,
            "radius mismatch dt={dt}"
        );
        assert!(
            (r * fdot + v * gdot - velocity).magnitude() / velocity.magnitude().max(1.0) < 2e-8
        );
    }
}

#[test]
fn independent_high_precision_reference_states_match() {
    let references = [
        (
            "stellar",
            Vector3::new(1.5e11, 0.0, 0.0),
            Vector3::new(0.0, 3.0e4, 500.0),
            1.327_124_400_18e20,
            1.0e7,
            Vector3::new(-58821995114.294509, 141934141370.56387, 2365569022.842731),
            Vector3::new(-27241.40531646277, -10770.02786881656, -179.500464480276),
        ),
        (
            "hyperbolic",
            Vector3::new(7.0e6, 0.0, 0.0),
            Vector3::new(-1800.0, 13000.0, 300.0),
            3.986_004_418e14,
            250000.0,
            Vector3::new(-1280362550.8234519, 1459953989.9982846, 33691245.92303734),
            Vector3::new(-5092.71238548234, 5735.973582150873, 132.3686211265586),
        ),
        (
            "nearly_parabolic",
            Vector3::new(7.0e6, 0.0, 0.0),
            Vector3::new(100.0, 10671.730905260201, 0.0),
            3.986_004_418e14,
            1.0e7,
            Vector3::new(-5688200135.448602, 520363940.812098, 0.0),
            Vector3::new(-386.10209163461267, 22.18829974624402, 0.0),
        ),
    ];
    for (name, r0, v0, mu, dt, expected_r, expected_v) in references {
        let (p, v) = propagate(r0, v0, mu, dt);
        let pr = (p - expected_r).magnitude() / expected_r.magnitude();
        let pv = (v - expected_v).magnitude() / expected_v.magnitude();
        assert!(
            pr < 2e-10,
            "{name} high precision position relative err {pr}"
        );
        assert!(
            pv < 2e-10,
            "{name} high precision velocity relative err {pv}"
        );
    }
}

#[test]
fn multi_revolution_and_substeps_agree() {
    let mu: f64 = 3.986_004_418e14;
    let r = Vector3::new(7.0e6, 0.0, 0.0);
    let v = Vector3::new(200.0, 6500.0, 100.0);
    let total = 55_000.0;
    let (p, w) = propagate(r, v, mu, total);
    let (mut rp, mut rv) = (r, v);
    for _ in 0..55 {
        (rp, rv) = propagate(rp, rv, mu, 1000.0);
    }
    assert!((p - rp).magnitude() / r.magnitude() < 1e-7);
    assert!((w - rv).magnitude() / v.magnitude() < 1e-7);
    assert_conserved(r, v, p, w, mu);
}
#[test]
fn invalid_arguments_never_produce_a_plausible_root() {
    let r = Vector3::new(7e6, 0.0, 0.0);
    let v = Vector3::new(0.0, 7800.0, 0.0);
    assert!(solve_universal_kepler(r, v, f64::NAN, 100.0).is_err());
    assert!(solve_universal_kepler(r, v, 3.986e14, f64::INFINITY).is_err());
    assert!(!stumpff_c(f64::NAN).is_finite());
    assert!(!stumpff_s(f64::INFINITY).is_finite());
}
